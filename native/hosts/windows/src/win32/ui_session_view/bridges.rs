//! Host-owned menu, notification, and window-service bridges for a UI session.
//!
//! Each method consumes only its session's mailbox and leaves native execution to
//! the owning UI thread. No native handle crosses this boundary.

use super::*;

impl UiSessionView {
    /// Attaches this session's window-title bridge and its validated name.
    ///
    /// A builder rather than another `new` parameter: the two values arrive
    /// together, only a registered session has them, and `new` already carries
    /// as many resources as one signature usefully can.
    #[must_use]
    pub(in crate::win32) fn with_window_title(
        mut self,
        mailbox: WindowTitleMailbox,
        display_name: impl Into<String>,
    ) -> Self {
        self.window_title = Some(mailbox);
        self.display_name = Some(display_name.into());
        self
    }

    /// Attaches this session's closed presentation-state bridge.
    #[must_use]
    pub(in crate::win32) fn with_window_state(mut self, mailbox: WindowStateMailbox) -> Self {
        self.window_state = Some(mailbox);
        self
    }

    /// Attaches this session's pull-only presentation-state observation bridge.
    #[must_use]
    pub(in crate::win32) fn with_window_state_read(
        mut self,
        mailbox: WindowStateReadMailbox,
    ) -> Self {
        self.window_state_read = Some(mailbox);
        self
    }

    /// Attaches this session's coalesced pull-only state-change mailbox.
    #[must_use]
    pub(in crate::win32) fn with_window_state_changes(
        mut self,
        mailbox: WindowStateChangesMailbox,
    ) -> Self {
        self.window_state_changes = Some(mailbox);
        self
    }

    /// Attaches this session's guarded foreground-request bridge.
    #[must_use]
    pub(in crate::win32) fn with_window_focus(mut self, mailbox: WindowFocusMailbox) -> Self {
        self.window_focus = Some(mailbox);
        self
    }

    /// Attaches this session's guarded reversible-fullscreen bridge.
    #[must_use]
    pub(in crate::win32) fn with_window_fullscreen(
        mut self,
        mailbox: WindowFullscreenMailbox,
    ) -> Self {
        self.window_fullscreen = Some(mailbox);
        self
    }

    /// Attaches this session's bounded logical client-size bridge.
    #[must_use]
    pub(in crate::win32) fn with_window_size(mut self, mailbox: WindowSizeMailbox) -> Self {
        self.window_size = Some(mailbox);
        self
    }

    /// Attaches this session's one-request native-menu bridge.
    #[must_use]
    pub(in crate::win32) fn with_menu(mut self, mailbox: MenuMailbox) -> Self {
        self.menu_mailbox = Some(mailbox);
        self
    }

    /// Attaches this session's one-request native context-menu bridge.
    #[must_use]
    pub(in crate::win32) fn with_context_menu(mut self, mailbox: ContextMenuMailbox) -> Self {
        self.context_menu_mailbox = Some(mailbox);
        self
    }

    /// Takes one pending validated context-menu replacement for this UI thread.
    pub(in crate::win32) fn take_context_menu_request(&self) -> Option<ContextMenuRequest> {
        self.context_menu_mailbox.as_ref()?.take()
    }

    /// Atomically retains one complete host-built context-menu model.
    ///
    /// Native construction completed before the registry lock was acquired, so
    /// a failed build leaves the prior model and future popup route intact.
    pub(in crate::win32) fn replace_context_menu(&mut self, next: ContextMenu) -> bool {
        self.context_menu = Some(next);
        true
    }

    /// Completes one context-menu replacement after the UI thread retained it.
    pub(in crate::win32) fn complete_context_menu_request(
        &self,
        request_id: u64,
        applied: bool,
    ) -> bool {
        let Some(mailbox) = self.context_menu_mailbox.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Clones the current host-retained context menu for a local popup route.
    pub(in crate::win32) fn context_menu(&self) -> Option<ContextMenu> {
        self.context_menu.clone()
    }

    /// Offers a selected current context-menu action to the shared queue.
    pub(in crate::win32) fn offer_context_menu_candidate(
        &self,
        candidate: anodrel_ui_session::ContextMenuInputCandidate,
    ) -> bool {
        if self.context_menu.is_none() {
            return false;
        }
        self.input_mailbox.push(candidate);
        true
    }

    /// Takes one pending validated menu replacement for this UI thread.
    pub(in crate::win32) fn take_menu_request(&self) -> Option<MenuRequest> {
        self.menu_mailbox.as_ref()?.take()
    }

    /// Attaches a fully constructed native menu and retires the prior bar.
    ///
    /// Construction occurred before the registry lock was acquired. A failed
    /// attachment therefore leaves the existing mapping and visible bar intact.
    pub(in crate::win32) fn attach_menu(&mut self, window: Hwnd, next: UnattachedMenu) -> bool {
        let Some(next) = next.attach(window) else {
            return false;
        };
        if let Some(previous) = self.menu_bar.replace(next) {
            previous.destroy_after_replacement();
        }
        true
    }

    /// Completes one menu replacement after this UI thread applied it.
    pub(in crate::win32) fn complete_menu_request(&self, request_id: u64, applied: bool) -> bool {
        let Some(mailbox) = self.menu_mailbox.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Offers a candidate only for this bar's current private normal-menu ID.
    pub(in crate::win32) fn offer_menu_command(&self, wparam: Wparam, lparam: Lparam) -> bool {
        let Some(candidate) = self
            .menu_bar
            .as_ref()
            .and_then(|bar| bar.candidate_from_command(wparam, lparam))
        else {
            return false;
        };
        self.input_mailbox.push(candidate);
        true
    }

    /// Offers a candidate only for this bar's current enabled local shortcut.
    pub(in crate::win32) fn offer_menu_shortcut(
        &self,
        key: Wparam,
        control_down: bool,
        shift_down: bool,
        alt_down: bool,
    ) -> bool {
        let Some(candidate) = self
            .menu_bar
            .as_ref()
            .and_then(|bar| bar.candidate_from_shortcut(key, control_down, shift_down, alt_down))
        else {
            return false;
        };
        self.input_mailbox.push(candidate);
        true
    }

    /// Takes a pending title proposal and composes the caption to apply.
    ///
    /// Composition happens here, on the side that holds the validated name, so
    /// the value handed to User32 is never one an application chose outright.
    pub(in crate::win32) fn take_window_title_request(&self) -> Option<(u64, String)> {
        let request = self.window_title.as_ref()?.take()?;
        let caption = anodrel_window::compose(request.proposal(), self.display_name.as_deref());
        Some((request.id(), caption))
    }

    /// Completes a title proposal after the host UI thread returns from User32.
    pub(in crate::win32) fn complete_window_title_request(
        &self,
        request_id: u64,
        applied: bool,
    ) -> bool {
        let Some(mailbox) = self.window_title.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Takes one closed state command for this window's owning UI thread.
    pub(in crate::win32) fn take_window_state_request(&self) -> Option<(u64, WindowState)> {
        let request = self.window_state.as_ref()?.take()?;
        Some((request.id(), request.state()))
    }

    /// Completes a state command after the host UI thread applies it.
    pub(in crate::win32) fn complete_window_state_request(
        &self,
        request_id: u64,
        applied: bool,
    ) -> bool {
        let Some(mailbox) = self.window_state.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Takes one pending state observation for this window's owning UI thread.
    pub(in crate::win32) fn take_window_state_read_request(&self) -> Option<u64> {
        Some(self.window_state_read.as_ref()?.take()?.id())
    }

    /// Completes one state observation with the UI thread's immediate sample.
    pub(in crate::win32) fn complete_window_state_read_request(
        &self,
        request_id: u64,
        state: Option<WindowState>,
    ) -> bool {
        let Some(mailbox) = self.window_state_read.as_ref() else {
            return false;
        };
        state.map_or_else(
            || mailbox.fail(request_id),
            |state| mailbox.complete(request_id, state),
        )
    }

    /// Records one UI-thread state observation in this view's bounded mailbox.
    ///
    /// The mailbox itself establishes a baseline and coalesces later values,
    /// so this bridge never allocates an event queue or starts a receiver.
    pub(in crate::win32) fn record_window_state_change(&self, state: WindowState) -> bool {
        let Some(mailbox) = self.window_state_changes.as_ref() else {
            return false;
        };
        mailbox.record_state(state);
        true
    }

    /// Takes one pending foreground request for this window's owning UI thread.
    pub(in crate::win32) fn take_window_focus_request(&self) -> Option<u64> {
        Some(self.window_focus.as_ref()?.take()?.id())
    }

    /// Completes a foreground request after the host UI thread asks Windows.
    pub(in crate::win32) fn complete_window_focus_request(
        &self,
        request_id: u64,
        requested: bool,
    ) -> bool {
        let Some(mailbox) = self.window_focus.as_ref() else {
            return false;
        };
        if requested {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Takes one pending reversible fullscreen mode for this window's UI thread.
    pub(in crate::win32) fn take_window_fullscreen_request(
        &self,
    ) -> Option<(u64, WindowFullscreenMode)> {
        let request = self.window_fullscreen.as_ref()?.take()?;
        Some((request.id(), request.mode()))
    }

    /// Returns a private copy of the presentation facts retained for restore.
    pub(in crate::win32) fn fullscreen_restore(
        &self,
    ) -> Option<crate::win32::fullscreen::FullscreenRestore> {
        self.fullscreen_restore.clone()
    }

    /// Replaces the private presentation facts after one host-side transition.
    ///
    /// This deliberately does not consult the protocol request: operating
    /// system state must remain recoverable even if the matching worker timed
    /// out just as the UI thread finished the native call.
    pub(in crate::win32) fn set_fullscreen_restore(
        &mut self,
        restore: Option<crate::win32::fullscreen::FullscreenRestore>,
    ) {
        self.fullscreen_restore = restore;
    }

    /// Completes one fullscreen request after the host applies its transition.
    pub(in crate::win32) fn complete_window_fullscreen_request(
        &self,
        request_id: u64,
        applied: bool,
    ) -> bool {
        let Some(mailbox) = self.window_fullscreen.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Takes one bounded client-size request for this window's owning UI thread.
    pub(in crate::win32) fn take_window_size_request(&self) -> Option<(u64, WindowSize)> {
        let request = self.window_size.as_ref()?.take()?;
        Some((request.id(), request.size()))
    }

    /// Completes one client-size request after the native transition returns.
    pub(in crate::win32) fn complete_window_size_request(
        &self,
        request_id: u64,
        applied: bool,
    ) -> bool {
        let Some(mailbox) = self.window_size.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Takes a pending notification for the host UI thread.
    ///
    /// The entry is returned alongside so the Shell32 call happens outside the
    /// window registry's lock.
    pub(in crate::win32) fn take_notification_request(
        &self,
    ) -> Option<(NotificationRequest, Option<Arc<WindowsNotifications>>)> {
        let request = self.notifications.take()?;
        Some((request, self.notification_entry.clone()))
    }

    /// Records the entry this session created on its first notification.
    pub(in crate::win32) fn set_notification_entry(&mut self, entry: Arc<WindowsNotifications>) {
        self.notification_entry = Some(entry);
    }

    /// Completes a notification after the host UI thread returns from Shell32.
    pub(in crate::win32) fn complete_notification_request(
        &self,
        request_id: u64,
        shown: bool,
    ) -> bool {
        if shown {
            self.notifications.complete(request_id)
        } else {
            self.notifications.fail(request_id)
        }
    }
}
