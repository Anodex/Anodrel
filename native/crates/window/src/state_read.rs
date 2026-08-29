//! Bounded, pull-only observation of one session window's presentation state.
//!
//! The worker asks once; the owning UI thread samples its own window and
//! answers with one closed portable value. The bridge has no subscription,
//! callback, target, geometry, or native-handle surface.

use std::time::Duration;

use crate::{
    WindowState, WindowStateReadServiceError,
    bridge::{WINDOW_COMMAND_RESPONSE_TIMEOUT, WindowMailbox},
};

/// Maximum time a protocol worker may wait for one state snapshot.
pub const WINDOW_STATE_READ_RESPONSE_TIMEOUT: Duration = WINDOW_COMMAND_RESPONSE_TIMEOUT;

/// A bounded pending state observation taken once by the host UI thread.
#[derive(Clone, Debug)]
pub struct WindowStateReadRequest {
    id: u64,
}

impl WindowStateReadRequest {
    /// Returns the identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
}

/// The portable service boundary for one pull-only state observation.
pub trait WindowStateReadService: std::fmt::Debug + Send {
    /// Samples the current standard state of this session's own window.
    ///
    /// The result is only minimized, maximized, or restored at the instant the
    /// owning UI thread samples it. It does not expose a target, timestamp,
    /// geometry, focus, fullscreen state, or future change notification.
    fn read_state(&self) -> Result<WindowState, WindowStateReadServiceError>;
}

/// A one-observation bridge from a protocol worker to one host UI thread.
#[derive(Clone, Debug, Default)]
pub struct WindowStateReadMailbox {
    bridge: WindowMailbox<(), WindowState>,
}

impl WindowStateReadMailbox {
    /// Creates an empty UI-thread state-observation mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the current observation once so the owning UI thread can sample.
    #[must_use]
    pub fn take(&self) -> Option<WindowStateReadRequest> {
        self.bridge
            .take()
            .map(|request| WindowStateReadRequest { id: request.id() })
    }

    /// Returns one state sampled by the host UI thread for the matching request.
    pub fn complete(&self, request_id: u64, state: WindowState) -> bool {
        self.bridge.complete_with(request_id, state)
    }

    /// Completes the matching observation with a safe unavailable result.
    pub fn fail(&self, request_id: u64) -> bool {
        self.bridge.fail(request_id)
    }

    #[cfg(test)]
    fn read_within(&self, timeout: Duration) -> Result<WindowState, WindowStateReadServiceError> {
        self.bridge.request_within((), timeout)
    }
}

impl WindowStateReadService for WindowStateReadMailbox {
    fn read_state(&self) -> Result<WindowState, WindowStateReadServiceError> {
        self.bridge
            .request_within((), WINDOW_STATE_READ_RESPONSE_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::{WindowStateReadMailbox, WindowStateReadService, WindowStateReadServiceError};
    use crate::WindowState;

    fn take_pending(mailbox: &WindowStateReadMailbox) -> super::WindowStateReadRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn transfers_one_closed_snapshot_once() {
        let mailbox = WindowStateReadMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read_state());

        let request = take_pending(&mailbox);
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(request.id(), WindowState::Maximized));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Ok(WindowState::Maximized)
        );
    }

    #[test]
    fn observations_are_busy_until_the_ui_thread_answers() {
        let mailbox = WindowStateReadMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read_state());

        let request = take_pending(&mailbox);
        assert_eq!(mailbox.read_state(), Err(WindowStateReadServiceError::Busy));
        assert!(mailbox.complete(request.id(), WindowState::Restored));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Ok(WindowState::Restored)
        );
    }

    #[test]
    fn a_late_or_untaken_completion_cannot_answer_an_observation() {
        let mailbox = WindowStateReadMailbox::new();
        assert!(!mailbox.complete(1, WindowState::Minimized));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read_state());
        let request = take_pending(&mailbox);
        assert!(!mailbox.complete(request.id().saturating_add(1), WindowState::Maximized));
        assert!(mailbox.fail(request.id()));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowStateReadServiceError::Unavailable)
        );
    }

    #[test]
    fn a_timed_out_observation_frees_the_session_again() {
        let mailbox = WindowStateReadMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read_within(Duration::from_millis(20)));

        let _abandoned = take_pending(&mailbox);
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowStateReadServiceError::Unavailable)
        );

        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.read_state());
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id(), WindowState::Minimized));
        assert_eq!(
            next.join().expect("the worker did not panic"),
            Ok(WindowState::Minimized)
        );
    }
}
