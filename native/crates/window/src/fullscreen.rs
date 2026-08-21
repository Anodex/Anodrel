//! The bounded UI-thread bridge for one session-owned fullscreen command.

use std::time::Duration;

use crate::{
    WindowFullscreenMode, WindowFullscreenServiceError,
    bridge::{WINDOW_COMMAND_RESPONSE_TIMEOUT, WindowCommandMailbox, WindowCommandRequest},
};

/// Maximum time a protocol worker may wait for a host UI thread to apply mode.
pub const WINDOW_FULLSCREEN_RESPONSE_TIMEOUT: Duration = WINDOW_COMMAND_RESPONSE_TIMEOUT;

/// A bounded pending fullscreen command taken once by the host UI thread.
#[derive(Clone, Debug)]
pub struct WindowFullscreenRequest {
    inner: WindowCommandRequest<WindowFullscreenMode>,
}

impl WindowFullscreenRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.inner.id()
    }

    /// Returns the closed presentation mode the host UI thread must apply.
    #[must_use]
    pub const fn mode(&self) -> WindowFullscreenMode {
        *self.inner.value()
    }
}

/// The portable service boundary for a fullscreen request on the session window.
pub trait WindowFullscreenService: std::fmt::Debug + Send {
    /// Applies one closed fullscreen mode to the session's own window.
    ///
    /// The mode reaches the operating system only through the host's owning UI
    /// thread. It carries no target and success reports no resulting desktop or
    /// window state.
    fn set_fullscreen(
        &self,
        mode: WindowFullscreenMode,
    ) -> Result<(), WindowFullscreenServiceError>;
}

/// A one-request bridge from a protocol worker to one host UI thread.
#[derive(Clone, Debug, Default)]
pub struct WindowFullscreenMailbox {
    bridge: WindowCommandMailbox<WindowFullscreenMode>,
}

impl WindowFullscreenMailbox {
    /// Creates an empty UI-thread window-fullscreen mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bridge: WindowCommandMailbox::new(),
        }
    }

    /// Takes the current fullscreen command once so the UI thread can apply it.
    #[must_use]
    pub fn take(&self) -> Option<WindowFullscreenRequest> {
        self.bridge
            .take()
            .map(|inner| WindowFullscreenRequest { inner })
    }

    /// Reports that the host UI thread applied the fullscreen command.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn complete(&self, request_id: u64) -> bool {
        self.bridge.complete(request_id)
    }

    /// Completes the matching request with a safe unavailable result.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn fail(&self, request_id: u64) -> bool {
        self.bridge.fail(request_id)
    }

    #[cfg(test)]
    fn set_within(
        &self,
        mode: WindowFullscreenMode,
        timeout: Duration,
    ) -> Result<(), WindowFullscreenServiceError> {
        self.bridge.request_within(mode, timeout)
    }
}

impl WindowFullscreenService for WindowFullscreenMailbox {
    fn set_fullscreen(
        &self,
        mode: WindowFullscreenMode,
    ) -> Result<(), WindowFullscreenServiceError> {
        self.bridge
            .request_within(mode, WINDOW_FULLSCREEN_RESPONSE_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::{WindowFullscreenMailbox, WindowFullscreenService, WindowFullscreenServiceError};
    use crate::WindowFullscreenMode;

    fn take_pending(mailbox: &WindowFullscreenMailbox) -> super::WindowFullscreenRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn transfers_one_closed_mode_and_reports_acceptance() {
        let mailbox = WindowFullscreenMailbox::new();
        let worker = mailbox.clone();
        let waiting =
            thread::spawn(move || worker.set_fullscreen(WindowFullscreenMode::Fullscreen));

        let request = take_pending(&mailbox);
        assert_eq!(request.mode(), WindowFullscreenMode::Fullscreen);
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn fullscreen_commands_are_busy_until_the_ui_thread_answers() {
        let mailbox = WindowFullscreenMailbox::new();
        let worker = mailbox.clone();
        let waiting =
            thread::spawn(move || worker.set_fullscreen(WindowFullscreenMode::Fullscreen));

        let request = take_pending(&mailbox);
        assert_eq!(
            mailbox.set_fullscreen(WindowFullscreenMode::Windowed),
            Err(WindowFullscreenServiceError::Busy)
        );
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn a_late_or_untaken_completion_cannot_answer_a_fullscreen_request() {
        let mailbox = WindowFullscreenMailbox::new();
        assert!(!mailbox.complete(1));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_fullscreen(WindowFullscreenMode::Windowed));
        let request = take_pending(&mailbox);
        assert!(!mailbox.complete(request.id().saturating_add(1)));
        assert!(mailbox.fail(request.id()));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowFullscreenServiceError::Unavailable)
        );
    }

    #[test]
    fn a_timed_out_fullscreen_command_frees_the_session_again() {
        let mailbox = WindowFullscreenMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || {
            worker.set_within(WindowFullscreenMode::Fullscreen, Duration::from_millis(20))
        });

        let _abandoned = take_pending(&mailbox);
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowFullscreenServiceError::Unavailable)
        );

        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.set_fullscreen(WindowFullscreenMode::Windowed));
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id()));
        assert_eq!(next.join().expect("the worker did not panic"), Ok(()));
    }
}
