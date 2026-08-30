//! One-request bridge from a session worker to its owning native UI thread.
//!
//! It moves only a validated complete context-menu model and its host-owned
//! revision. Local popup placement, native menus, command IDs, and operating
//! system calls remain exclusively on the UI-thread side.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crate::{ContextMenuModel, ContextMenuRevision, ContextMenuService, ContextMenuServiceError};

/// Maximum time a worker may wait for one native context-menu replacement.
pub const CONTEXT_MENU_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// One validated replacement taken exactly once by the owning UI thread.
#[derive(Clone, Debug)]
pub struct ContextMenuRequest {
    id: u64,
    revision: ContextMenuRevision,
    model: ContextMenuModel,
}

impl ContextMenuRequest {
    /// Returns the identity used only to complete this bridge entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the opaque monotonic revision the host assigned to this model.
    #[must_use]
    pub const fn revision(&self) -> ContextMenuRevision {
        self.revision
    }

    /// Returns the already validated complete model.
    #[must_use]
    pub fn model(&self) -> &ContextMenuModel {
        &self.model
    }
}

/// One pending context-menu replacement shared by a worker and one UI thread.
///
/// It holds at most one complete model. A timed-out request clears only itself,
/// so a late UI completion cannot answer a future replacement.
#[derive(Clone, Debug, Default)]
pub struct ContextMenuMailbox {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: Option<ActiveRequest>,
}

#[derive(Debug)]
struct ActiveRequest {
    request: ContextMenuRequest,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    result: Mutex<Option<Result<(), ContextMenuServiceError>>>,
    ready: Condvar,
}

impl ContextMenuMailbox {
    /// Creates an empty session-owned context-menu bridge.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gives the owning UI thread the current replacement exactly once.
    #[must_use]
    pub fn take(&self) -> Option<ContextMenuRequest> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request.clone())
        }
    }

    /// Reports that the owning UI thread accepted the replacement.
    pub fn complete(&self, request_id: u64) -> bool {
        self.respond(request_id, Ok(()))
    }

    /// Reports that the owning UI thread could not retain the replacement.
    pub fn fail(&self, request_id: u64) -> bool {
        self.respond(request_id, Err(ContextMenuServiceError::Unavailable))
    }

    fn request_within(
        &self,
        revision: ContextMenuRevision,
        model: ContextMenuModel,
        timeout: Duration,
    ) -> Result<(), ContextMenuServiceError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let state = &mut *lock(&self.state);
            if state.active.is_some() {
                return Err(ContextMenuServiceError::Unavailable);
            }
            state.next_id = state.next_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_id;
            state.active = Some(ActiveRequest {
                request: ContextMenuRequest {
                    id: request_id,
                    revision,
                    model,
                },
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        let result = wait_for_response(&response, timeout);
        if result.is_none() {
            let state = &mut *lock(&self.state);
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.request.id == request_id)
            {
                state.active = None;
            }
        }
        result.unwrap_or(Err(ContextMenuServiceError::Unavailable))
    }

    fn respond(&self, request_id: u64, result: Result<(), ContextMenuServiceError>) -> bool {
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
        let result_slot = &mut *lock(&response.result);
        *result_slot = Some(result);
        response.ready.notify_one();
        true
    }
}

impl ContextMenuService for ContextMenuMailbox {
    fn replace(
        &self,
        revision: ContextMenuRevision,
        model: ContextMenuModel,
    ) -> Result<(), ContextMenuServiceError> {
        self.request_within(revision, model, CONTEXT_MENU_RESPONSE_TIMEOUT)
    }
}

fn wait_for_response(
    response: &ResponseSlot,
    timeout: Duration,
) -> Option<Result<(), ContextMenuServiceError>> {
    let result = lock(&response.result);
    let (mut result, timed_out) = response
        .ready
        .wait_timeout_while(result, timeout, |result| result.is_none())
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if timed_out.timed_out() && result.is_none() {
        None
    } else {
        result.take()
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    use super::{CONTEXT_MENU_RESPONSE_TIMEOUT, ContextMenuMailbox};
    use crate::{
        ContextMenuModel, ContextMenuRevision, ContextMenuService, ContextMenuServiceError,
        MenuAction, MenuActionId, MenuText,
    };

    fn model() -> ContextMenuModel {
        ContextMenuModel::new(vec![MenuAction::new(
            MenuActionId::new("document.rename").expect("fixed ID is valid"),
            MenuText::new("Rename").expect("fixed label is valid"),
            true,
        )])
        .expect("fixed model is valid")
    }

    fn revision() -> ContextMenuRevision {
        ContextMenuRevision::INITIAL
            .next()
            .expect("the first revision exists")
    }

    #[test]
    fn transfers_one_complete_model_to_the_ui_thread_and_awaits_completion() {
        let mailbox = ContextMenuMailbox::new();
        let worker = mailbox.clone();
        let (sent, received) = mpsc::channel();
        let waiting = thread::spawn(move || {
            sent.send(worker.replace(revision(), model()).map(|()| "completed"))
        });

        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.revision(), revision());
        assert_eq!(request.model(), &model());
        assert!(mailbox.complete(request.id()));
        assert_eq!(received.recv().expect("worker answer"), Ok("completed"));
        waiting
            .join()
            .expect("worker joined")
            .expect("worker sent its result");
    }

    #[test]
    fn rejects_a_second_replacement_while_one_is_pending_or_taken() {
        let mailbox = ContextMenuMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.replace(revision(), model()));
        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };

        assert_eq!(
            mailbox.replace(revision(), model()),
            Err(ContextMenuServiceError::Unavailable)
        );
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("worker joined"), Ok(()));
    }

    #[test]
    fn rejects_untaken_stale_and_timed_out_completions() {
        let mailbox = ContextMenuMailbox::new();
        assert!(!mailbox.complete(1));
        assert!(!mailbox.fail(1));
        assert_eq!(
            mailbox.request_within(revision(), model(), Duration::ZERO),
            Err(ContextMenuServiceError::Unavailable)
        );
        assert!(mailbox.take().is_none());
        assert!(!mailbox.complete(1));
        assert!(Instant::now().elapsed() < CONTEXT_MENU_RESPONSE_TIMEOUT);
    }
}
