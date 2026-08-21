//! One-request bridge from a session worker to its owning native UI thread.
//!
//! The bridge moves only a validated complete menu model and its host-owned
//! revision. It deliberately has no window, handle, command identifier, or
//! operating-system dependency; the direct Windows host supplies those on the
//! UI-thread side.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crate::{MenuModel, MenuRevision, MenuService, MenuServiceError};

/// Maximum time a session worker may wait for one native menu replacement.
///
/// Constructing and attaching this bounded menu is a small UI-thread action.
/// A longer wait would only strand the authenticated worker when its owning
/// window is gone or its UI thread has stopped responding.
pub const MENU_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// One validated replacement taken exactly once by the owning UI thread.
#[derive(Clone, Debug)]
pub struct MenuRequest {
    id: u64,
    revision: MenuRevision,
    model: MenuModel,
}

impl MenuRequest {
    /// Returns the identity used only to complete this bridge entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the opaque monotonic revision the host assigned to this model.
    #[must_use]
    pub const fn revision(&self) -> MenuRevision {
        self.revision
    }

    /// Returns the already validated complete model.
    #[must_use]
    pub fn model(&self) -> &MenuModel {
        &self.model
    }
}

/// One pending menu replacement shared by a session worker and one UI thread.
///
/// It holds at most one complete model. The UI thread must take it, build and
/// attach the host-owned native menu, then answer the exact request. The
/// service clears a timed-out request by identity so a late completion cannot
/// answer a subsequent replacement.
#[derive(Clone, Debug, Default)]
pub struct MenuMailbox {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: Option<ActiveRequest>,
}

#[derive(Debug)]
struct ActiveRequest {
    request: MenuRequest,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    result: Mutex<Option<Result<(), MenuServiceError>>>,
    ready: Condvar,
}

impl MenuMailbox {
    /// Creates an empty session-owned menu bridge.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gives the owning UI thread the current replacement exactly once.
    #[must_use]
    pub fn take(&self) -> Option<MenuRequest> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request.clone())
        }
    }

    /// Reports that the owning UI thread attached the replacement.
    ///
    /// Returns `false` for an untaken, stale, or already answered request.
    pub fn complete(&self, request_id: u64) -> bool {
        self.respond(request_id, Ok(()))
    }

    /// Reports that the owning UI thread could not attach the replacement.
    ///
    /// Returns `false` for an untaken, stale, or already answered request.
    pub fn fail(&self, request_id: u64) -> bool {
        self.respond(request_id, Err(MenuServiceError::Unavailable))
    }

    fn request_within(
        &self,
        revision: MenuRevision,
        model: MenuModel,
        timeout: Duration,
    ) -> Result<(), MenuServiceError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let state = &mut *lock(&self.state);
            if state.active.is_some() {
                return Err(MenuServiceError::Unavailable);
            }
            state.next_id = state.next_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_id;
            state.active = Some(ActiveRequest {
                request: MenuRequest {
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
            // Clear only this slot. A UI thread may have taken its request just
            // before the timeout, but it must never complete a later one.
            let state = &mut *lock(&self.state);
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.request.id == request_id)
            {
                state.active = None;
            }
        }
        result.unwrap_or(Err(MenuServiceError::Unavailable))
    }

    fn respond(&self, request_id: u64, result: Result<(), MenuServiceError>) -> bool {
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

impl MenuService for MenuMailbox {
    fn replace(&self, revision: MenuRevision, model: MenuModel) -> Result<(), MenuServiceError> {
        self.request_within(revision, model, MENU_RESPONSE_TIMEOUT)
    }
}

fn wait_for_response(
    response: &ResponseSlot,
    timeout: Duration,
) -> Option<Result<(), MenuServiceError>> {
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

    use super::{MENU_RESPONSE_TIMEOUT, MenuMailbox};
    use crate::{Menu, MenuAction, MenuActionId, MenuModel, MenuRevision, MenuService, MenuText};

    fn model() -> MenuModel {
        MenuModel::new(vec![
            Menu::new(
                MenuText::new("File").expect("fixed label is valid"),
                vec![MenuAction::new(
                    MenuActionId::new("document.new").expect("fixed ID is valid"),
                    MenuText::new("New document").expect("fixed label is valid"),
                    true,
                )],
            )
            .expect("fixed menu is valid"),
        ])
        .expect("fixed model is valid")
    }

    fn revision() -> MenuRevision {
        MenuRevision::INITIAL
            .next()
            .expect("the first revision exists")
    }

    #[test]
    fn transfers_one_complete_model_to_the_ui_thread_and_awaits_completion() {
        let mailbox = MenuMailbox::new();
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
        let mailbox = MenuMailbox::new();
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
            Err(crate::MenuServiceError::Unavailable)
        );
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("worker joined"), Ok(()));
    }

    #[test]
    fn rejects_untaken_and_stale_completions() {
        let mailbox = MenuMailbox::new();
        assert!(!mailbox.complete(1));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.replace(revision(), model()));
        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };
        assert!(!mailbox.complete(request.id().saturating_add(1)));
        assert!(mailbox.fail(request.id()));
        assert_eq!(
            waiting.join().expect("worker joined"),
            Err(crate::MenuServiceError::Unavailable)
        );
    }

    #[test]
    fn a_timeout_clears_only_its_own_request_and_never_sticks_the_mailbox() {
        let mailbox = MenuMailbox::new();
        let started = Instant::now();
        assert_eq!(
            mailbox.request_within(revision(), model(), Duration::ZERO),
            Err(crate::MenuServiceError::Unavailable)
        );
        assert!(started.elapsed() < MENU_RESPONSE_TIMEOUT);
        assert!(mailbox.take().is_none());
        assert!(!mailbox.complete(1));
    }
}
