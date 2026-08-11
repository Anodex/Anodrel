//! One bounded, host-UI-thread window-title bridge.
//!
//! A window caption is set through User32, and the rule that keeps a pipe
//! worker away from User32 applies here as it does to Shell32. The
//! authenticated worker hands one validated proposal to this mailbox and the
//! owning native UI thread performs the call.
//!
//! Deliberately the same shape as the notification bridge in
//! `anodrel-notifications`: one pending request, taken exactly once, answered
//! by the UI thread, with a timeout that frees the session rather than leaving
//! it busy forever. Two bridges that behave alike are two bridges a reader only
//! has to understand once.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crate::{WindowTitleProposal, WindowTitleService, WindowTitleServiceError};

/// Maximum time a protocol worker may wait for its host UI thread to respond.
///
/// Matches the notification bridge. Applying a caption is a single fast call,
/// so a worker blocked longer than this is not being patient, it is stuck.
pub const WINDOW_TITLE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// A bounded pending proposal taken exactly once by the host UI thread.
#[derive(Clone, Debug)]
pub struct WindowTitleRequest {
    id: u64,
    proposal: WindowTitleProposal,
}

impl WindowTitleRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the validated proposal the host UI thread must apply.
    ///
    /// This is the application's half only. The UI thread composes the caption
    /// with the validated display name it holds; see [`crate::compose`].
    #[must_use]
    pub const fn proposal(&self) -> &WindowTitleProposal {
        &self.proposal
    }
}

/// A one-request bridge from a protocol worker to one host UI thread.
#[derive(Clone, Debug, Default)]
pub struct WindowTitleMailbox {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: Option<ActiveRequest>,
}

#[derive(Debug)]
struct ActiveRequest {
    request: WindowTitleRequest,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    value: Mutex<Option<Result<(), WindowTitleServiceError>>>,
    ready: Condvar,
}

impl WindowTitleMailbox {
    /// Creates an empty UI-thread window-title mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the current proposal once so the owning UI thread can apply it.
    #[must_use]
    pub fn take(&self) -> Option<WindowTitleRequest> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request.clone())
        }
    }

    /// Reports that the host UI thread applied the caption.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn complete(&self, request_id: u64) -> bool {
        self.respond(request_id, Ok(()))
    }

    /// Completes the matching request with a safe unavailable result.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn fail(&self, request_id: u64) -> bool {
        self.respond(request_id, Err(WindowTitleServiceError::Unavailable))
    }

    fn set_within(
        &self,
        proposal: &WindowTitleProposal,
        timeout: Duration,
    ) -> Result<(), WindowTitleServiceError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let state = &mut *lock(&self.state);
            if state.active.is_some() {
                return Err(WindowTitleServiceError::Busy);
            }
            state.next_id = state.next_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_id;
            state.active = Some(ActiveRequest {
                request: WindowTitleRequest {
                    id: request_id,
                    proposal: proposal.clone(),
                },
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        let result = wait_for_response(&response, timeout);
        if result.is_none() {
            // The UI thread never answered. Clear the slot so this session is
            // not left permanently busy by one stuck request.
            let state = &mut *lock(&self.state);
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.request.id == request_id)
            {
                state.active = None;
            }
        }
        result.unwrap_or(Err(WindowTitleServiceError::Unavailable))
    }

    fn respond(&self, request_id: u64, result: Result<(), WindowTitleServiceError>) -> bool {
        let response = {
            let state = &mut *lock(&self.state);
            let Some(active) = state.active.as_ref() else {
                return false;
            };
            if active.request.id != request_id || !active.taken {
                return false;
            }
            let response = Arc::clone(&active.response);
            state.active = None;
            response
        };
        let value = &mut *lock(&response.value);
        *value = Some(result);
        response.ready.notify_one();
        true
    }
}

impl WindowTitleService for WindowTitleMailbox {
    fn set_title(&self, proposal: &WindowTitleProposal) -> Result<(), WindowTitleServiceError> {
        self.set_within(proposal, WINDOW_TITLE_RESPONSE_TIMEOUT)
    }
}

fn wait_for_response(
    response: &ResponseSlot,
    timeout: Duration,
) -> Option<Result<(), WindowTitleServiceError>> {
    let value = lock(&response.value);
    let (mut value, timed_out) = response
        .ready
        .wait_timeout_while(value, timeout, |value| value.is_none())
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if timed_out.timed_out() && value.is_none() {
        None
    } else {
        value.take()
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, thread, time::Duration};

    use super::{
        WindowTitleMailbox, WindowTitleRequest, WindowTitleService, WindowTitleServiceError,
    };
    use crate::WindowTitleProposal;

    fn proposal() -> WindowTitleProposal {
        WindowTitleProposal::new("Quarterly Report.pdf").expect("the test proposal is valid")
    }

    /// Spins until the UI-thread side can take the pending request.
    fn take_pending(mailbox: &WindowTitleMailbox) -> WindowTitleRequest {
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
        // A request is handed to the UI thread exactly once.
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
        // Busy and Unavailable are different answers: one means try again, the
        // other means this host has no window to title at all.
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
        // Completing before the UI thread has taken it would let a response
        // race ahead of the call it is supposed to describe.
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

        // The session must not be left permanently busy by one stuck request.
        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.set_title(&proposal()));
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id()));
        assert_eq!(next.join().expect("the worker did not panic"), Ok(()));
    }
}
