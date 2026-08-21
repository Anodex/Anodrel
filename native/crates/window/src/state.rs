//! The bounded UI-thread bridge for one session-owned presentation command.

use std::time::Duration;

use crate::{
    WindowState, WindowStateServiceError,
    bridge::{WINDOW_COMMAND_RESPONSE_TIMEOUT, WindowCommandMailbox, WindowCommandRequest},
};

/// Maximum time a protocol worker may wait for a host UI thread to apply state.
pub const WINDOW_STATE_RESPONSE_TIMEOUT: Duration = WINDOW_COMMAND_RESPONSE_TIMEOUT;

/// A bounded pending presentation command taken once by the host UI thread.
#[derive(Clone, Debug)]
pub struct WindowStateRequest {
    inner: WindowCommandRequest<WindowState>,
}

impl WindowStateRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.inner.id()
    }

    /// Returns the closed state the host UI thread must apply.
    #[must_use]
    pub const fn state(&self) -> WindowState {
        *self.inner.value()
    }
}

/// The portable service boundary for a state request on the session's window.
pub trait WindowStateService: std::fmt::Debug + Send {
    /// Applies one closed presentation state to the session's own window.
    ///
    /// The state reaches the operating system only through the host's owning UI
    /// thread. It carries no target, and success reports no resulting state.
    fn set_state(&self, state: WindowState) -> Result<(), WindowStateServiceError>;
}

/// A one-request bridge from a protocol worker to one host UI thread.
#[derive(Clone, Debug, Default)]
pub struct WindowStateMailbox {
    bridge: WindowCommandMailbox<WindowState>,
}

impl WindowStateMailbox {
    /// Creates an empty UI-thread window-state mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bridge: WindowCommandMailbox::new(),
        }
    }

    /// Takes the current state command once so the owning UI thread can apply it.
    #[must_use]
    pub fn take(&self) -> Option<WindowStateRequest> {
        self.bridge.take().map(|inner| WindowStateRequest { inner })
    }

    /// Reports that the host UI thread applied the state command.
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
        state: WindowState,
        timeout: Duration,
    ) -> Result<(), WindowStateServiceError> {
        self.bridge.request_within(state, timeout)
    }
}

impl WindowStateService for WindowStateMailbox {
    fn set_state(&self, state: WindowState) -> Result<(), WindowStateServiceError> {
        self.bridge
            .request_within(state, WINDOW_STATE_RESPONSE_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::{WindowStateMailbox, WindowStateService, WindowStateServiceError};
    use crate::WindowState;

    fn take_pending(mailbox: &WindowStateMailbox) -> super::WindowStateRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn transfers_a_closed_state_once_and_reports_acceptance() {
        let mailbox = WindowStateMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_state(WindowState::Maximized));

        let request = take_pending(&mailbox);
        assert_eq!(request.state(), WindowState::Maximized);
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn state_commands_are_busy_until_the_ui_thread_answers() {
        let mailbox = WindowStateMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_state(WindowState::Minimized));

        let request = take_pending(&mailbox);
        assert_eq!(
            mailbox.set_state(WindowState::Restored),
            Err(WindowStateServiceError::Busy)
        );
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn a_late_or_untaken_completion_cannot_answer_a_state_request() {
        let mailbox = WindowStateMailbox::new();
        assert!(!mailbox.complete(1));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_state(WindowState::Restored));
        let request = take_pending(&mailbox);
        assert!(!mailbox.complete(request.id().saturating_add(1)));
        assert!(mailbox.fail(request.id()));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowStateServiceError::Unavailable)
        );
    }

    #[test]
    fn a_timed_out_state_command_frees_the_session_again() {
        let mailbox = WindowStateMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || {
            worker.set_within(WindowState::Minimized, Duration::from_millis(20))
        });

        let _abandoned = take_pending(&mailbox);
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowStateServiceError::Unavailable)
        );

        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.set_state(WindowState::Restored));
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id()));
        assert_eq!(next.join().expect("the worker did not panic"), Ok(()));
    }
}
