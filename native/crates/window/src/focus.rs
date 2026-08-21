//! The bounded UI-thread bridge for one session-owned foreground request.

use std::time::Duration;

use crate::{
    WindowFocusServiceError,
    bridge::{WINDOW_COMMAND_RESPONSE_TIMEOUT, WindowCommandMailbox, WindowCommandRequest},
};

/// Maximum time a protocol worker may wait for a host UI thread to request focus.
pub const WINDOW_FOCUS_RESPONSE_TIMEOUT: Duration = WINDOW_COMMAND_RESPONSE_TIMEOUT;

/// A bounded pending foreground request taken once by the host UI thread.
#[derive(Clone, Debug)]
pub struct WindowFocusRequest {
    inner: WindowCommandRequest<()>,
}

impl WindowFocusRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.inner.id()
    }
}

/// The portable service boundary for a foreground request on the session's window.
pub trait WindowFocusService: std::fmt::Debug + Send {
    /// Asks the operating system to foreground this session's own window.
    ///
    /// The request reaches the operating system only through the host's owning
    /// UI thread. It carries no target, and success reports no resulting focus
    /// or activation state.
    fn request_focus(&self) -> Result<(), WindowFocusServiceError>;
}

/// A one-request bridge from a protocol worker to one host UI thread.
#[derive(Clone, Debug, Default)]
pub struct WindowFocusMailbox {
    bridge: WindowCommandMailbox<()>,
}

impl WindowFocusMailbox {
    /// Creates an empty UI-thread window-focus mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bridge: WindowCommandMailbox::new(),
        }
    }

    /// Takes the current focus request once so the owning UI thread can apply it.
    #[must_use]
    pub fn take(&self) -> Option<WindowFocusRequest> {
        self.bridge.take().map(|inner| WindowFocusRequest { inner })
    }

    /// Reports that the host UI thread asked Windows to foreground this window.
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
    fn request_within(&self, timeout: Duration) -> Result<(), WindowFocusServiceError> {
        self.bridge.request_within((), timeout)
    }
}

impl WindowFocusService for WindowFocusMailbox {
    fn request_focus(&self) -> Result<(), WindowFocusServiceError> {
        self.bridge
            .request_within((), WINDOW_FOCUS_RESPONSE_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::{WindowFocusMailbox, WindowFocusService, WindowFocusServiceError};

    fn take_pending(mailbox: &WindowFocusMailbox) -> super::WindowFocusRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn transfers_one_focus_request_and_reports_acceptance() {
        let mailbox = WindowFocusMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.request_focus());

        let request = take_pending(&mailbox);
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn focus_requests_are_busy_until_the_ui_thread_answers() {
        let mailbox = WindowFocusMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.request_focus());

        let request = take_pending(&mailbox);
        assert_eq!(mailbox.request_focus(), Err(WindowFocusServiceError::Busy));
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn a_late_or_untaken_completion_cannot_answer_a_focus_request() {
        let mailbox = WindowFocusMailbox::new();
        assert!(!mailbox.complete(1));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.request_focus());
        let request = take_pending(&mailbox);
        assert!(!mailbox.complete(request.id().saturating_add(1)));
        assert!(mailbox.fail(request.id()));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowFocusServiceError::Unavailable)
        );
    }

    #[test]
    fn a_timed_out_focus_request_frees_the_session_again() {
        let mailbox = WindowFocusMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.request_within(Duration::from_millis(20)));

        let _abandoned = take_pending(&mailbox);
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowFocusServiceError::Unavailable)
        );

        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.request_focus());
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id()));
        assert_eq!(next.join().expect("the worker did not panic"), Ok(()));
    }
}
