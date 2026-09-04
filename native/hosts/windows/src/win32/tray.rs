//! Direct User32 popup construction for one semantic notification-area tray.
//!
//! Shell32 owns callback delivery and User32 owns the short-lived popup. This
//! module translates neither surface into an application callback, handle,
//! coordinate, command number, or window control API.

use std::{collections::BTreeMap, ptr, sync::Arc};

use anodrel_menu::{MenuActionId, TrayRequest, TrayRevision};
use anodrel_ui_session::TrayInputCandidate;

use super::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, Hmenu, Hwnd, Point, PostMessageW,
    SetForegroundWindow, TrackPopupMenu, Uint, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, to_wide_null,
};

/// The private range for at most sixteen notification-area popup commands.
const FIRST_TRAY_COMMAND_ID: u16 = 0x7200;
const MF_GRAYED: Uint = 0x0001;
const TPM_RETURNCMD: Uint = 0x0100;
const TPM_RIGHTBUTTON: Uint = 0x0002;

/// One complete host-retained tray model and private command mapping.
#[derive(Clone)]
pub(super) struct TrayMenu {
    inner: Arc<TrayMenuInner>,
}

struct TrayMenuInner {
    revision: TrayRevision,
    items: Vec<TrayItem>,
    enabled_actions: BTreeMap<u16, MenuActionId>,
}

struct TrayItem {
    command: u16,
    label: String,
    enabled: bool,
}

/// The private result of offering one native tray popup.
pub(super) enum TrayDisplay {
    /// The popup was dismissed without an enabled current command.
    Dismissed,
    /// A current enabled command became a revision-bound semantic candidate.
    Selected(TrayInputCandidate),
}

/// The only notification-area mouse inputs that this first slice accepts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TrayCallback {
    /// Windows reported a local right-button release.
    RightClick,
    /// Windows reported a local left-button release.
    LeftClick,
}

impl TrayMenu {
    /// Builds and probes a complete native popup before retaining the model.
    ///
    /// The probe lets a failed User32 allocation leave the current model and
    /// notification-area entry untouched.
    pub(super) fn build(request: &TrayRequest) -> Option<Self> {
        let mut next_command = FIRST_TRAY_COMMAND_ID;
        let mut items = Vec::with_capacity(request.model().items().len());
        let mut enabled_actions = BTreeMap::new();
        for action in request.model().items() {
            let command = next_command;
            next_command = next_command.checked_add(1)?;
            if action.enabled() {
                enabled_actions.insert(command, action.id().clone());
            }
            items.push(TrayItem {
                command,
                label: action.label().as_str().to_owned(),
                enabled: action.enabled(),
            });
        }
        let menu = Self {
            inner: Arc::new(TrayMenuInner {
                revision: request.revision(),
                items,
                enabled_actions,
            }),
        };
        let _ = PopupMenu::build(&menu)?;
        Some(menu)
    }

    /// Opens one host-owned popup at the current shell-selected cursor point.
    pub(super) fn show(&self, window: Hwnd) -> Option<TrayDisplay> {
        let mut point = Point::default();
        // SAFETY: `point` is writable stack storage. Shell32 delivered the
        // callback immediately before this call, so Windows retains the
        // current local cursor position; it never crosses a host boundary.
        if unsafe { GetCursorPos(&mut point) } == 0 {
            return None;
        }
        let popup = PopupMenu::build(self)?;
        // SAFETY: the complete popup, current cursor point, and owning window
        // are all host-selected. The application supplied none of them.
        let command = unsafe {
            TrackPopupMenu(
                popup.handle,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                window,
                ptr::null(),
            )
        };
        // Shell32's documented tray-popup pattern posts WM_NULL after the
        // transient menu closes. It carries no data and its failure has no
        // semantic consequence.
        unsafe {
            PostMessageW(window, WM_NULL, 0, 0);
        }
        let candidate = u16::try_from(command)
            .ok()
            .and_then(|command| self.candidate_for_command(command));
        Some(match candidate {
            Some(candidate) => TrayDisplay::Selected(candidate),
            None => TrayDisplay::Dismissed,
        })
    }

    /// Maps only one current enabled private command to a semantic action.
    fn candidate_for_command(&self, command: u16) -> Option<TrayInputCandidate> {
        let action = self.inner.enabled_actions.get(&command)?.clone();
        Some(TrayInputCandidate::new(self.inner.revision, action))
    }
}

/// Decodes only the two local mouse messages a notification-area tray needs.
pub(super) const fn callback_from_lparam(lparam: isize) -> Option<TrayCallback> {
    match lparam as u32 {
        WM_RBUTTONUP => Some(TrayCallback::RightClick),
        WM_LBUTTONUP => Some(TrayCallback::LeftClick),
        _ => None,
    }
}

/// Routes one private Shell32 callback only when this view has a tray model.
///
/// The callback itself carries no application data. A right click creates a
/// temporary host popup, while a left click makes only a best-effort request
/// to foreground this same host window. Neither route reports a result.
pub(super) fn handle_callback(window: Hwnd, lparam: isize) -> bool {
    let Some(callback) = callback_from_lparam(lparam) else {
        return false;
    };
    let Ok(Some(tray)) = super::registry::tray(window) else {
        return false;
    };
    match callback {
        TrayCallback::RightClick => match tray.show(window) {
            Some(TrayDisplay::Selected(candidate)) => {
                let _ = super::registry::offer_tray_candidate(window, candidate);
                true
            }
            Some(TrayDisplay::Dismissed) => true,
            None => false,
        },
        TrayCallback::LeftClick => {
            // SAFETY: this is the exact host window whose retained tray model
            // accepted the local callback. Windows remains free to decline.
            unsafe {
                SetForegroundWindow(window);
            }
            true
        }
    }
}

/// One temporary User32 popup destroyed before a callback returns.
struct PopupMenu {
    handle: Hmenu,
}

impl PopupMenu {
    fn build(menu: &TrayMenu) -> Option<Self> {
        // SAFETY: CreatePopupMenu returns either a new empty menu or null.
        let handle = unsafe { CreatePopupMenu() };
        if handle == 0 {
            return None;
        }
        let popup = Self { handle };
        for item in &menu.inner.items {
            let label = to_wide_null(&escape_mnemonics(&item.label));
            let flags = if item.enabled { 0 } else { MF_GRAYED };
            // SAFETY: `popup` owns the menu, `command` is host-private, and
            // `label` remains a live null-terminated UTF-16 string.
            if unsafe {
                AppendMenuW(
                    popup.handle,
                    flags,
                    usize::from(item.command),
                    label.as_ptr(),
                )
            } == 0
            {
                return None;
            }
        }
        Some(popup)
    }
}

impl Drop for PopupMenu {
    fn drop(&mut self) {
        // SAFETY: every nonzero handle came from CreatePopupMenu and was never
        // attached to a window or another menu.
        unsafe { DestroyMenu(self.handle) };
    }
}

fn escape_mnemonics(value: &str) -> String {
    value.replace('&', "&&")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use anodrel_menu::{MenuActionId, TrayRevision};

    use super::{
        FIRST_TRAY_COMMAND_ID, TrayCallback, TrayMenu, TrayMenuInner, callback_from_lparam,
    };

    fn menu() -> TrayMenu {
        TrayMenu {
            inner: std::sync::Arc::new(TrayMenuInner {
                revision: TrayRevision::INITIAL
                    .next()
                    .expect("the first revision exists"),
                items: Vec::new(),
                enabled_actions: BTreeMap::from([(
                    FIRST_TRAY_COMMAND_ID,
                    MenuActionId::new("window.open").expect("fixed ID is valid"),
                )]),
            }),
        }
    }

    #[test]
    fn recognizes_only_local_left_and_right_click_messages() {
        assert_eq!(callback_from_lparam(0x0205), Some(TrayCallback::RightClick));
        assert_eq!(callback_from_lparam(0x0202), Some(TrayCallback::LeftClick));
        assert_eq!(callback_from_lparam(0x007B), None);
    }

    #[test]
    fn maps_only_a_current_enabled_private_command() {
        let candidate = menu()
            .candidate_for_command(FIRST_TRAY_COMMAND_ID)
            .expect("the private command is mapped");
        let (revision, action) = candidate.into_parts();
        assert_eq!(revision.value(), 1);
        assert_eq!(action.as_str(), "window.open");
        assert!(menu().candidate_for_command(0x7FFF).is_none());
    }
}
