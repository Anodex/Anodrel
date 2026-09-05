//! Native product-update system-menu action and safe terminal presentation.

use std::{ffi::OsStr, os::windows::ffi::OsStrExt};

use anodrel_windows_product_update::{
    ProductUpdateOutcome, ProductUpdatePoll, request_update_consent,
};
use anodrel_windows_taskbar_progress::set_taskbar_progress;

use super::super::{
    DrawMenuBar, EnableMenuItem, GetSystemMenu, Hwnd, MessageBoxW, Uint, Wparam, registry,
};

// Windows reserves the high command range for system-menu messages. The low
// four bits are ignored by WM_SYSCOMMAND, so this value is sixteen-aligned.
const SC_ANODREL_CHECK_UPDATES: Uint = 0xF1E0;
const SC_COMMAND_MASK: Uint = 0xFFF0;
const MF_GRAYED: Uint = 0x0001;
const MB_ICONINFORMATION: Uint = 0x0000_0040;
const MB_ICONWARNING: Uint = 0x0000_0030;
const MB_OK: Uint = 0x0000_0000;
const MENU_LABEL: &str = "Check for Anodrel updates";
const DIALOG_CAPTION: &str = "Anodrel update";
const INSTALLED_TEXT: &str =
    "An Anodrel update has been installed.\r\n\r\nRestart the application to use it.";
const FAILED_TEXT: &str = "Anodrel could not complete the update check.";

/// Adds the one fixed host-owned update action when this exact window has a
/// signed-policy update controller. No application model or normal menu bar is
/// involved, so application menu replacement cannot remove or forge it.
pub(in crate::win32) fn install_product_update_action(window: Hwnd, base_caption: &str) -> bool {
    if !registry::has_product_update_action(window).unwrap_or(false) {
        return false;
    }
    // SAFETY: `window` was created by this UI thread; Windows owns the returned
    // system-menu handle for the life of that window.
    let menu = unsafe { GetSystemMenu(window, 0) };
    if menu == 0 {
        return false;
    }
    let label = wide(MENU_LABEL);
    // SAFETY: the system-menu handle is valid while this window exists and the
    // UTF-16 label remains alive throughout the synchronous append call.
    let appended = unsafe {
        super::super::AppendMenuW(menu, 0, SC_ANODREL_CHECK_UPDATES as usize, label.as_ptr()) != 0
    };
    appended && registry::prepare_product_update_presentation(window, base_caption).unwrap_or(false)
}

/// Returns whether this system command is Anodrel's one product-update action.
pub(in crate::win32) const fn is_product_update_command(wparam: Wparam) -> bool {
    (wparam as Uint & SC_COMMAND_MASK) == SC_ANODREL_CHECK_UPDATES
}

/// Starts a native update discovery worker after the person's system-menu click.
pub(in crate::win32) fn start_product_update(window: Hwnd) {
    match registry::begin_product_update(window) {
        Ok(Some(true)) => {
            set_action_enabled(window, false);
            refresh_presentation(window);
        }
        Ok(Some(false) | None) => {}
        Err(_) => show(window, FAILED_TEXT, MB_ICONWARNING),
    }
}

/// Polls a running update without blocking the product window's UI thread.
///
/// The only blocking work here is the existing native consent dialog after a
/// signed offer completed discovery. Transfer, UAC waiting, and policy proof
/// stay on the controller's worker.
pub(in crate::win32) fn service_product_update(window: Hwnd) {
    let outcome = match registry::poll_product_update(window) {
        Ok(Some(ProductUpdatePoll::Pending)) | Ok(None) => None,
        Ok(Some(ProductUpdatePoll::Complete(outcome))) => Some(outcome),
        Ok(Some(ProductUpdatePoll::ConsentRequired(offer))) => {
            match request_update_consent(offer) {
                Ok(consent) => match registry::submit_product_update_consent(window, consent) {
                    Ok(Some(ProductUpdatePoll::Complete(outcome))) => Some(outcome),
                    Ok(Some(
                        ProductUpdatePoll::Pending | ProductUpdatePoll::ConsentRequired(_),
                    ))
                    | Ok(None) => None,
                    Err(_) => Some(ProductUpdateOutcome::Failed),
                },
                Err(_) => Some(ProductUpdateOutcome::Failed),
            }
        }
        Err(_) => Some(ProductUpdateOutcome::Failed),
    };
    let active = registry::product_update_is_active(window)
        .ok()
        .flatten()
        .unwrap_or(false);
    set_action_enabled(window, !active);
    refresh_presentation(window);
    if let Some(outcome) = outcome {
        present_terminal_outcome(window, outcome);
    }
}

/// Marks the product window taskbar available only after Windows sent the
/// documented `TaskbarButtonCreated` message for this exact window.
pub(in crate::win32) fn taskbar_button_created(window: Hwnd) {
    let change = registry::product_update_taskbar_button_created(window)
        .ok()
        .flatten();
    apply_presentation(window, change);
}

/// Drops optional taskbar readiness after the Shell recreated its taskbar.
///
/// The caption remains visible, and Windows will send a fresh button-created
/// message before any taskbar API call can resume.
pub(in crate::win32) fn taskbar_restarted(window: Hwnd) {
    let _ = registry::product_update_taskbar_restarted(window);
}

/// Clears an optional taskbar indicator while the product window still exists.
pub(in crate::win32) fn clear_product_update_taskbar(window: Hwnd) {
    if let Ok(Some(progress)) = registry::clear_product_update_taskbar(window) {
        let _ = set_taskbar_progress(window, progress);
    }
}

/// Applies the latest host-owned state, after every registry lock is released.
pub(in crate::win32) fn refresh_presentation(window: Hwnd) {
    let change = registry::refresh_product_update_presentation(window)
        .ok()
        .flatten();
    apply_presentation(window, change);
}

fn apply_presentation(window: Hwnd, change: Option<registry::ProductUpdatePresentationChange>) {
    let Some(change) = change else {
        return;
    };
    if let Some(caption) = change.caption {
        let caption = wide(&caption);
        // SAFETY: the caption is already host-composed and the UTF-16 buffer
        // remains live through this direct UI-thread call.
        unsafe {
            let _ = super::super::SetWindowTextW(window, caption.as_ptr());
        }
    }
    if let Some(progress) = change.taskbar {
        let _ = set_taskbar_progress(window, progress);
    }
}

fn present_terminal_outcome(window: Hwnd, outcome: ProductUpdateOutcome) {
    match outcome {
        ProductUpdateOutcome::Installed => show(window, INSTALLED_TEXT, MB_ICONINFORMATION),
        ProductUpdateOutcome::ConsentDeclined | ProductUpdateOutcome::ElevationDeclined => {}
        ProductUpdateOutcome::Failed => show(window, FAILED_TEXT, MB_ICONWARNING),
    }
}

fn set_action_enabled(window: Hwnd, enabled: bool) {
    // SAFETY: the system menu belongs to this UI-thread-owned window. The
    // command is one fixed Anodrel constant and `DrawMenuBar` only refreshes
    // this same native frame.
    unsafe {
        let menu = GetSystemMenu(window, 0);
        if menu == 0 {
            return;
        }
        let flags = if enabled { 0 } else { MF_GRAYED };
        let _ = EnableMenuItem(menu, SC_ANODREL_CHECK_UPDATES, flags);
        let _ = DrawMenuBar(window);
    }
}

fn show(window: Hwnd, text: &str, icon: Uint) {
    let text = wide(text);
    let caption = wide(DIALOG_CAPTION);
    // SAFETY: this product window owns the synchronous modal dialog; both
    // UTF-16 buffers remain alive for the duration and all text is fixed host
    // copy. The result is intentionally unobserved.
    unsafe {
        let _ = MessageBoxW(window, text.as_ptr(), caption.as_ptr(), MB_OK | icon);
    }
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{INSTALLED_TEXT, SC_ANODREL_CHECK_UPDATES, is_product_update_command};

    #[test]
    fn system_command_masks_only_windows_reserved_low_bits() {
        assert!(is_product_update_command(SC_ANODREL_CHECK_UPDATES as usize));
        assert!(is_product_update_command(
            (SC_ANODREL_CHECK_UPDATES | 0x000F) as usize
        ));
        assert!(!is_product_update_command(0xF170));
    }

    #[test]
    fn completion_copy_requires_restart_without_offering_an_automatic_action() {
        assert!(INSTALLED_TEXT.contains("Restart the application"));
        assert!(!INSTALLED_TEXT.contains("automatically"));
    }
}
