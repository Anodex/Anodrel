//! The bounded UI-thread bridge for one session-owned title proposal.

use std::time::Duration;

use crate::{
    WindowTitleProposal, WindowTitleService, WindowTitleServiceError,
    bridge::{WINDOW_COMMAND_RESPONSE_TIMEOUT, WindowCommandMailbox, WindowCommandRequest},
};

/// Maximum time a protocol worker may wait for a host UI thread to apply a title.
pub const WINDOW_TITLE_RESPONSE_TIMEOUT: Duration = WINDOW_COMMAND_RESPONSE_TIMEOUT;

/// A bounded pending proposal taken exactly once by the host UI thread.
#[derive(Clone, Debug)]
pub struct WindowTitleRequest {
    inner: WindowCommandRequest<WindowTitleProposal>,
}

impl WindowTitleRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.inner.id()
    }

    /// Returns the validated proposal the host UI thread must apply.
    ///
    /// This is the application's half only. The UI thread composes the caption
    /// with the validated display name it holds; see [`crate::compose`].
    #[must_use]
    pub const fn proposal(&self) -> &WindowTitleProposal {
        self.inner.value()
    }
}

/// A one-request bridge from a protocol worker to one host UI thread.
#[derive(Clone, Debug, Default)]
pub struct WindowTitleMailbox {
    bridge: WindowCommandMailbox<WindowTitleProposal>,
}

impl WindowTitleMailbox {
    /// Creates an empty UI-thread window-title mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bridge: WindowCommandMailbox::new(),
        }
    }

    /// Takes the current proposal once so the owning UI thread can apply it.
    #[must_use]
    pub fn take(&self) -> Option<WindowTitleRequest> {
        self.bridge.take().map(|inner| WindowTitleRequest { inner })
    }

    /// Reports that the host UI thread applied the caption.
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
        proposal: &WindowTitleProposal,
        timeout: Duration,
    ) -> Result<(), WindowTitleServiceError> {
        self.bridge.request_within(proposal.clone(), timeout)
    }
}

impl WindowTitleService for WindowTitleMailbox {
    fn set_title(&self, proposal: &WindowTitleProposal) -> Result<(), WindowTitleServiceError> {
        self.bridge
            .request_within(proposal.clone(), WINDOW_TITLE_RESPONSE_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, thread, time::Duration};

    use super::{WindowTitleMailbox, WindowTitleService, WindowTitleServiceError};
    use crate::WindowTitleProposal;

    fn proposal() -> WindowTitleProposal {
        WindowTitleProposal::new("Quarterly Report.pdf").expect("the test proposal is valid")
    }

    /// Spins until the UI-thread side can take the pending request.
    fn take_pending(mailbox: &WindowTitleMailbox) -> super::WindowTitleRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn transfers_one_proposal_to_the_ui_thread_and_reports_acceptance() {
        let mailbox = WindowTitleMailbox::new();
        let worker = mailbox.clone();
        let (sent, received) = mpsc::channel();
        let worker_thread = thread::spawn(move || sent.send(worker.set_title(&proposal())));

        let request = take_pending(&mailbox);
        assert_eq!(request.proposal(), &proposal());
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(request.id()));

        assert_eq!(received.recv().expect("the worker responded"), Ok(()));
        worker_thread
            .join()
            .expect("the worker did not panic")
            .expect("the receiver remained available");
    }

    #[test]
    fn refuses_a_second_proposal_as_busy_rather_than_unavailable() {
        let mailbox = WindowTitleMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_title(&proposal()));

        let request = take_pending(&mailbox);
        assert_eq!(
            mailbox.set_title(&proposal()),
            Err(WindowTitleServiceError::Busy)
        );
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn ignores_a_completion_for_an_untaken_or_unknown_request() {
        let mailbox = WindowTitleMailbox::new();
        assert!(!mailbox.complete(1));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.set_title(&proposal()));
        while mailbox.take().is_none() {
            thread::yield_now();
        }
        assert!(!mailbox.complete(9_999));
        assert!(mailbox.fail(1));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowTitleServiceError::Unavailable)
        );
    }

    #[test]
    fn a_request_the_ui_thread_never_answers_frees_the_session_again() {
        let mailbox = WindowTitleMailbox::new();
        let worker = mailbox.clone();
        let waiting =
            thread::spawn(move || worker.set_within(&proposal(), Duration::from_millis(20)));

        let _abandoned = take_pending(&mailbox);
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(WindowTitleServiceError::Unavailable)
        );

        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.set_title(&proposal()));
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id()));
        assert_eq!(next.join().expect("the worker did not panic"), Ok(()));
    }
}
