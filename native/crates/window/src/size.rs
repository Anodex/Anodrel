//! The bounded UI-thread bridge for one session-owned client-size command.

use std::time::Duration;

use crate::{
    WindowSize, WindowSizeServiceError,
    bridge::{WINDOW_COMMAND_RESPONSE_TIMEOUT, WindowCommandMailbox, WindowCommandRequest},
};

/// Maximum time a protocol worker may wait for a host UI thread to apply size.
pub const WINDOW_SIZE_RESPONSE_TIMEOUT: Duration = WINDOW_COMMAND_RESPONSE_TIMEOUT;

/// A bounded pending client-size command taken once by the host UI thread.
#[derive(Clone, Debug)]
pub struct WindowSizeRequest {
    inner: WindowCommandRequest<WindowSize>,
}

impl WindowSizeRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.inner.id()
    }

    /// Returns the validated logical client size the host UI thread must apply.
    #[must_use]
    pub const fn size(&self) -> WindowSize {
        *self.inner.value()
    }
}

/// The portable service boundary for a client-size request on the session window.
pub trait WindowSizeService: std::fmt::Debug + Send {
    /// Applies one bounded logical client size to the session's own window.
    ///
    /// The size reaches the operating system only through the host's owning UI
    /// thread. It carries no target, position, monitor, or geometry readback.
    fn set_size(&self, size: WindowSize) -> Result<(), WindowSizeServiceError>;
}

/// A one-request bridge from a protocol worker to one host UI thread.
#[derive(Clone, Debug, Default)]
pub struct WindowSizeMailbox {
    bridge: WindowCommandMailbox<WindowSize>,
}

impl WindowSizeMailbox {
    /// Creates an empty UI-thread window-size mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bridge: WindowCommandMailbox::new(),
        }
    }

    /// Takes the current client-size command once so the UI thread can apply it.
    #[must_use]
    pub fn take(&self) -> Option<WindowSizeRequest> {
        self.bridge.take().map(|inner| WindowSizeRequest { inner })
    }

    /// Reports that the host UI thread applied the client-size command.
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
        size: WindowSize,
        timeout: Duration,
    ) -> Result<(), WindowSizeServiceError> {
        self.bridge.request_within(size, timeout)
    }
}

impl WindowSizeService for WindowSizeMailbox {
    fn set_size(&self, size: WindowSize) -> Result<(), WindowSizeServiceError> {
        self.bridge
            .request_within(size, WINDOW_SIZE_RESPONSE_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::{WindowSizeMailbox, WindowSizeService, WindowSizeServiceError};
    use crate::WindowSize;

    fn size() -> WindowSize {
        WindowSize::new(800, 600).expect("fixture size is valid")
    }

    fn take_pending(mailbox: &WindowSizeMailbox) -> super::WindowSizeRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn transfers_one_bounded_size_and_reports_acceptance() {
        let mailbox = WindowSizeMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_size(size()));

        let request = take_pending(&mailbox);
        assert_eq!(request.size(), size());
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn size_commands_are_busy_until_the_ui_thread_answers() {
        let mailbox = WindowSizeMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_size(size()));

        let request = take_pending(&mailbox);
        assert_eq!(mailbox.set_size(size()), Err(WindowSizeServiceError::Busy));
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn a_late_or_untaken_completion_cannot_answer_a_size_request() {
        let mailbox = WindowSizeMailbox::new();
        assert!(!mailbox.complete(1));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_size(size()));
        let request = take_pending(&mailbox);
        assert!(!mailbox.complete(request.id().saturating_add(1)));
        assert!(mailbox.fail(request.id()));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowSizeServiceError::Unavailable)
        );
    }

    #[test]
    fn a_timed_out_size_command_frees_the_session_again() {
        let mailbox = WindowSizeMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_within(size(), Duration::from_millis(20)));

        let _abandoned = take_pending(&mailbox);
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowSizeServiceError::Unavailable)
        );

        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.set_size(size()));
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id()));
        assert_eq!(next.join().expect("the worker did not panic"), Ok(()));
    }
}
