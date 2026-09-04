//! Session-local tray mailbox and retained-model bridge methods.

use super::*;

impl UiSessionView {
    /// Attaches this session's one-request native notification-area tray bridge.
    #[must_use]
    pub(in crate::win32) fn with_tray(mut self, mailbox: TrayMailbox) -> Self {
        self.tray_mailbox = Some(mailbox);
        self
    }

    /// Takes one pending validated tray replacement for this UI thread.
    pub(in crate::win32) fn take_tray_request(&self) -> Option<TrayRequest> {
        self.tray_mailbox.as_ref()?.take()
    }

    /// Atomically retains one complete host-built tray mapping.
    ///
    /// Shell32 entry creation and User32 construction both finished before the
    /// registry lock was acquired, so a failed build leaves the last working
    /// native tray untouched.
    pub(in crate::win32) fn replace_tray(&mut self, next: super::super::tray::TrayMenu) -> bool {
        self.tray = Some(next);
        true
    }

    /// Completes one tray replacement after the UI thread retained it.
    pub(in crate::win32) fn complete_tray_request(&self, request_id: u64, applied: bool) -> bool {
        let Some(mailbox) = self.tray_mailbox.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Clones the current tray mapping for a later native callback.
    ///
    /// Native popup display occurs after the registry lock is released. Core
    /// revalidation discards an action if a newer replacement wins before the
    /// application reads it.
    pub(in crate::win32) fn tray(&self) -> Option<super::super::tray::TrayMenu> {
        self.tray.clone()
    }

    /// Offers one selected private tray command to the shared session queue.
    pub(in crate::win32) fn offer_tray_candidate(
        &self,
        candidate: anodrel_ui_session::TrayInputCandidate,
    ) -> bool {
        if self.tray.is_none() {
            return false;
        }
        self.input_mailbox.push(candidate);
        true
    }
}
