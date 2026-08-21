//! Direct User32 menu construction and private command mapping.
//!
//! This module is the only place where Anodrel's semantic session menu becomes
//! native User32 menu objects. It keeps those objects, their numeric command
//! identifiers, and `WM_COMMAND` filtering on the owning UI thread.

use std::{
    collections::BTreeMap,
    mem,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anodrel_menu::{MenuActionId, MenuRequest, MenuRevision};
use anodrel_ui_session::MenuInputCandidate;

use super::{
    AppendMenuW, CreateMenu, CreatePopupMenu, DestroyMenu, DrawMenuBar, Hmenu, Hwnd, Lparam,
    SetMenu, Uint, Wparam, to_wide_null,
};

/// The host-private range for the at-most-128 command items in one session.
///
/// Application semantic IDs never select these values. The range is only a
/// small table key used while this exact host-created menu is attached.
const FIRST_COMMAND_ID: u16 = 0x7000;
const MF_GRAYED: Uint = 0x0001;
const MF_POPUP: Uint = 0x0010;

/// One constructed but not yet window-attached menu bar.
///
/// Dropping an unattached menu frees it. Once attached, Windows owns its
/// lifetime until it is replaced or the window closes.
pub(super) struct UnattachedMenu {
    handle: Hmenu,
    revision: MenuRevision,
    actions: BTreeMap<u16, MenuActionId>,
}

impl UnattachedMenu {
    /// Builds all native objects for one already validated complete model.
    ///
    /// Nothing has reached a window if this returns `None`; the caller can
    /// therefore retain the previous visible menu without a rollback path.
    pub(super) fn build(request: &MenuRequest) -> Option<Self> {
        // SAFETY: CreateMenu requires no input and returns either a new empty
        // User32 menu or a null handle.
        let handle = unsafe { CreateMenu() };
        if handle == 0 {
            return None;
        }
        let mut built = Self {
            handle,
            revision: request.revision(),
            actions: BTreeMap::new(),
        };
        let mut next_command = FIRST_COMMAND_ID;
        for menu in request.model().menus() {
            // SAFETY: CreatePopupMenu requires no input and returns either a
            // new empty User32 menu or a null handle.
            let popup = unsafe { CreatePopupMenu() };
            if popup == 0 {
                return None;
            }
            for action in menu.items() {
                let command = next_command;
                let Some(next) = next_command.checked_add(1) else {
                    // The portable model caps the count far below this limit,
                    // but fail safely if an invariant ever changes.
                    // SAFETY: the popup has not been attached to another menu.
                    unsafe { DestroyMenu(popup) };
                    return None;
                };
                let label = to_wide_null(&escape_mnemonics(action.label().as_str()));
                let flags = if action.enabled() { 0 } else { MF_GRAYED };
                // SAFETY: `popup` is this thread's new menu, `command` is a
                // host-private value, and `label` is live null-terminated UTF-16.
                if unsafe { AppendMenuW(popup, flags, usize::from(command), label.as_ptr()) } == 0 {
                    // SAFETY: the popup is still unattached on this path.
                    unsafe { DestroyMenu(popup) };
                    return None;
                }
                built.actions.insert(command, action.id().clone());
                next_command = next;
            }
            let label = to_wide_null(&escape_mnemonics(menu.label().as_str()));
            // SAFETY: `popup` is an unattached completed submenu, `built.handle`
            // is the new top-level menu, and `label` is live null-terminated UTF-16.
            if unsafe { AppendMenuW(built.handle, MF_POPUP, popup as usize, label.as_ptr()) } == 0 {
                // SAFETY: a popup becomes owned by the root only after the
                // successful MF_POPUP append, so it remains ours here.
                unsafe { DestroyMenu(popup) };
                return None;
            }
        }
        Some(built)
    }

    /// Attaches this complete menu to one host-owned top-level window.
    ///
    /// A failed `SetMenu` leaves this value unattached, so dropping it destroys
    /// only the attempted replacement. `SetMenu` itself redraws the window;
    /// `DrawMenuBar` additionally requests the normal menu-frame refresh but
    /// is not a semantic second attachment step.
    pub(super) fn attach(mut self, window: Hwnd) -> Option<MenuBar> {
        // SAFETY: the window belongs to this UI thread and `self.handle` is a
        // fully constructed top-level User32 menu owned by this call.
        if unsafe { SetMenu(window, self.handle) } == 0 {
            return None;
        }
        // SAFETY: Windows documents this as the repaint operation after a menu
        // changes. SetMenu already succeeded, so a repaint request failure does
        // not change the currently attached menu or its host mapping.
        unsafe { DrawMenuBar(window) };
        let handle = mem::replace(&mut self.handle, 0);
        Some(MenuBar {
            inner: Arc::new(MenuBarInner {
                handle,
                revision: self.revision,
                actions: mem::take(&mut self.actions),
                destroyed: AtomicBool::new(false),
            }),
        })
    }
}

impl Drop for UnattachedMenu {
    fn drop(&mut self) {
        if self.handle != 0 {
            // SAFETY: an attached menu is disarmed by `attach`; every remaining
            // nonzero handle is an unattached root owned only by this value.
            unsafe { DestroyMenu(self.handle) };
        }
    }
}

/// One menu bar currently attached to a host-owned window.
///
/// It intentionally has no `Drop` implementation. Windows frees a menu still
/// assigned to a closing window; a replacement explicitly destroys the prior
/// bar only after `SetMenu` has made it unattached.
#[derive(Clone)]
pub(super) struct MenuBar {
    inner: Arc<MenuBarInner>,
}

struct MenuBarInner {
    handle: Hmenu,
    revision: MenuRevision,
    actions: BTreeMap<u16, MenuActionId>,
    destroyed: AtomicBool,
}

impl MenuBar {
    /// Derives one revision-bound candidate only from the normal menu-message
    /// shape documented by User32.
    pub(super) fn candidate_from_command(
        &self,
        wparam: Wparam,
        lparam: Lparam,
    ) -> Option<MenuInputCandidate> {
        let notification = (wparam >> 16) & usize::from(u16::MAX);
        if notification != 0 || lparam != 0 {
            return None;
        }
        let command = u16::try_from(wparam & usize::from(u16::MAX)).ok()?;
        let action = self.inner.actions.get(&command)?.clone();
        Some(MenuInputCandidate::new(self.inner.revision, action))
    }

    /// Frees a menu after another menu replaced it on the same window.
    pub(super) fn destroy_after_replacement(&self) {
        // SAFETY: the caller invokes this only after a successful SetMenu made
        // this handle no longer the window's current menu. The marker is shared
        // with paint snapshots, so a cloned view can never free it twice.
        if !self.inner.destroyed.swap(true, Ordering::AcqRel) {
            unsafe { DestroyMenu(self.inner.handle) };
        }
    }
}

/// Escapes every User32 mnemonic marker in application display text.
///
/// Labels are semantic display values, never accelerator declarations. Doubling
/// an ampersand is the direct Win32 spelling for a literal ampersand.
fn escape_mnemonics(value: &str) -> String {
    value.replace('&', "&&")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use anodrel_menu::{MenuActionId, MenuRevision};

    use super::{FIRST_COMMAND_ID, MenuBar, escape_mnemonics};

    fn bar() -> MenuBar {
        MenuBar {
            inner: std::sync::Arc::new(super::MenuBarInner {
                handle: 0,
                revision: MenuRevision::INITIAL
                    .next()
                    .expect("the first revision exists"),
                actions: BTreeMap::from([(
                    FIRST_COMMAND_ID,
                    MenuActionId::new("document.new").expect("fixed ID is valid"),
                )]),
                destroyed: std::sync::atomic::AtomicBool::new(false),
            }),
        }
    }

    #[test]
    fn escapes_application_ampersands_without_claiming_a_mnemonic() {
        assert_eq!(escape_mnemonics("Save & close"), "Save && close");
        assert_eq!(escape_mnemonics("&&"), "&&&&");
    }

    #[test]
    fn accepts_only_a_current_private_normal_menu_command() {
        let candidate = bar()
            .candidate_from_command(usize::from(FIRST_COMMAND_ID), 0)
            .expect("the private command is mapped");
        let (revision, action) = candidate.into_parts();
        assert_eq!(revision.value(), 1);
        assert_eq!(action.as_str(), "document.new");
    }

    #[test]
    fn refuses_accelerator_control_and_unknown_command_shapes() {
        let bar = bar();
        assert!(
            bar.candidate_from_command(usize::from(FIRST_COMMAND_ID) | (1_usize << 16), 0)
                .is_none()
        );
        assert!(
            bar.candidate_from_command(usize::from(FIRST_COMMAND_ID), 1)
                .is_none()
        );
        assert!(bar.candidate_from_command(0x7FFF, 0).is_none());
    }
}
