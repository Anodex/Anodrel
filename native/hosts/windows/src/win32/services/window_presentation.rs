//! Targetless presentation commands for one registry-owned session window.

use super::*;
use anodrel_window::WindowState;

/// Converts the portable closed state into its one documented User32 command.
///
/// Keeping this conversion separate makes the native boundary exhaustive and
/// unit-testable without creating a window. No caller can supply the integer.
pub(crate) const fn presentation_command(state: WindowState) -> i32 {
    match state {
        WindowState::Minimized => SW_MINIMIZE,
        WindowState::Maximized => SW_MAXIMIZE,
        WindowState::Restored => SW_RESTORE,
    }
}

/// Reduces the two documented User32 standard-state facts to the portable
/// closed vocabulary. A minimized window wins because Windows can retain a
/// maximized restored placement while it is currently iconic.
pub(crate) const fn observed_presentation_state(minimized: bool, maximized: bool) -> WindowState {
    if minimized {
        WindowState::Minimized
    } else if maximized {
        WindowState::Maximized
    } else {
        WindowState::Restored
    }
}

/// Applies one pending window title for a session window, if it has one.
///
/// The `SetWindowTextW` call runs outside the window registry's lock, matching
/// how a notification is serviced: a caption change is fast, but nothing that
/// calls into User32 should hold a lock every other window's message handling
/// waits on.
///
/// The caption arriving here is already composed — the application's proposal
/// plus the session's validated display name — so this function has no way to
/// apply a title an application chose outright. See `docs/WINDOW_TITLE.md`.
pub(crate) fn service_window_title(window: Hwnd) {
    let Ok(Some((request_id, caption))) = registry::take_window_title_request(window) else {
        return;
    };
    let caption = to_wide_null(&caption);
    // SAFETY: this runs on the thread that created the window, and `caption` is
    // a null-terminated UTF-16 buffer that outlives the call.
    let applied = unsafe { SetWindowTextW(window, caption.as_ptr()) } != 0;
    let _ = registry::complete_window_title_request(window, request_id, applied);
}

/// Applies one pending closed presentation state for a session window.
///
/// `ShowWindow` returns the previous visibility, not an error indicator, so a
/// request that reaches this known UI-thread-owned window is complete after the
/// call. There is deliberately no state query before or after it: returning
/// either fact would turn a write-only command into window-state readback.
pub(crate) fn service_window_state(window: Hwnd) {
    let Ok(Some((request_id, state))) = registry::take_window_state_request(window) else {
        return;
    };
    let command = presentation_command(state);
    // SAFETY: this runs on the thread that created `window`, and `command` is
    // one of the three documented User32 presentation-state values above.
    unsafe { ShowWindow(window, command) };
    let _ = registry::complete_window_state_request(window, request_id, true);
}

/// Answers one pull-only standard-state observation for this session window.
///
/// The only native facts sampled are the two documented standard-state flags
/// of the same UI-thread-owned window. Neither flag, a handle, nor a timing
/// detail crosses the registry boundary; the worker receives only one closed
/// value through its own separately granted mailbox.
pub(crate) fn service_window_state_read(window: Hwnd) {
    let Ok(Some(request_id)) = registry::take_window_state_read_request(window) else {
        return;
    };
    // SAFETY: this runs on the thread that created `window`, which is the only
    // host-selected window associated with this request. The calls inspect no
    // other window and return only User32 boolean state flags.
    let state =
        unsafe { observed_presentation_state(IsIconic(window) != 0, IsZoomed(window) != 0) };
    let _ = registry::complete_window_state_read_request(window, request_id, Some(state));
}

/// Asks Windows to foreground this session window for one pending request.
///
/// This runs on the thread that created `window` and obtains the target only
/// from that window's registry entry. It does not observe the prior foreground
/// window, call `AllowSetForegroundWindow`, synthesize input, or retry a
/// refusal. Windows remains authoritative over foreground policy.
pub(crate) fn service_window_focus(window: Hwnd) {
    let Ok(Some(request_id)) = registry::take_window_focus_request(window) else {
        return;
    };
    // SAFETY: this runs on the thread that created `window`, and `window` is
    // resolved solely from that session's host-owned view registry entry.
    let requested = unsafe { SetForegroundWindow(window) } != 0;
    let _ = registry::complete_window_focus_request(window, request_id, requested);
}

/// Applies one pending reversible fullscreen mode for a session window.
///
/// All registry access is deliberately short and ends before calling User32:
/// frame changes can synchronously re-enter this window procedure. Restore
/// facts are recorded before entry and cleared only after a completed native
/// restore, so a worker timeout cannot strand a borderless window without its
/// original placement.
pub(crate) fn service_window_fullscreen(window: Hwnd) {
    let Ok(Some((request_id, mode, saved))) = registry::take_window_fullscreen_request(window)
    else {
        return;
    };

    let applied = match (mode, saved) {
        // The host's retained record makes this duplicate request idempotent
        // without a native state query or a protocol-visible state read.
        (WindowFullscreenMode::Fullscreen, Some(_)) => true,
        (WindowFullscreenMode::Fullscreen, None) => match fullscreen::capture(window) {
            None => false,
            Some(saved) => {
                if registry::set_window_fullscreen_restore(window, Some(saved.clone()))
                    .ok()
                    .flatten()
                    .is_none()
                {
                    false
                } else {
                    match fullscreen::enter(window, &saved) {
                        fullscreen::FullscreenEntry::Applied => true,
                        fullscreen::FullscreenEntry::Windowed => {
                            let _ = registry::set_window_fullscreen_restore(window, None);
                            false
                        }
                        fullscreen::FullscreenEntry::RestorePending => false,
                    }
                }
            }
        },
        // Exiting a window that the host has not entered is also idempotent.
        (WindowFullscreenMode::Windowed, None) => true,
        (WindowFullscreenMode::Windowed, Some(saved)) => {
            let restored = fullscreen::restore(window, &saved);
            if restored {
                let _ = registry::set_window_fullscreen_restore(window, None);
            }
            restored
        }
    };
    let _ = registry::complete_window_fullscreen_request(window, request_id, applied);
}

/// Applies one pending bounded client-size request on its owning UI thread.
///
/// Fullscreen transitions run immediately before this route. While the host
/// holds a private restore fact, the request fails instead of altering the
/// geometry that restoration must recover.
pub(crate) fn service_window_size(window: Hwnd) {
    let Ok(Some((request_id, size, fullscreen_active))) =
        registry::take_window_size_request(window)
    else {
        return;
    };
    let applied = !fullscreen_active && size::apply(window, size);
    let _ = registry::complete_window_size_request(window, request_id, applied);
}
