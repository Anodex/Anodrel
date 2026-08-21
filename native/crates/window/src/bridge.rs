//! Shared, one-request bridge from a protocol worker to one host UI thread.
//!
//! Each public window service wraps this generic mechanism in its own closed
//! request type. Keeping the synchronisation here avoids several similar
//! bridges gradually disagreeing about completion, timeout, or lock order.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crate::WindowCommandError;

/// Maximum time a protocol worker may wait for a host UI-thread response.
///
/// Applying a title or one standard presentation state is fast. Waiting longer
/// would not make the operation more likely to succeed; it would only leave a
/// session worker stuck behind an unavailable UI thread.
pub(crate) const WINDOW_COMMAND_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// One closed command taken exactly once by the owning UI thread.
#[derive(Clone, Debug)]
pub(crate) struct WindowCommandRequest<T> {
    id: u64,
    value: T,
}

impl<T> WindowCommandRequest<T> {
    /// Returns the identity used only to complete this bridge entry.
    pub(crate) const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the already-validated command value.
    pub(crate) const fn value(&self) -> &T {
        &self.value
    }
}

/// One pending command shared by a session worker and its owning UI thread.
#[derive(Clone, Debug)]
pub(crate) struct WindowCommandMailbox<T> {
    state: Arc<Mutex<State<T>>>,
}

#[derive(Debug)]
struct State<T> {
    next_id: u64,
    active: Option<ActiveRequest<T>>,
}

#[derive(Debug)]
struct ActiveRequest<T> {
    request: WindowCommandRequest<T>,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    value: Mutex<Option<Result<(), WindowCommandError>>>,
    ready: Condvar,
}

impl<T> Default for WindowCommandMailbox<T> {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                next_id: 0,
                active: None,
            })),
        }
    }
}

impl<T: Clone> WindowCommandMailbox<T> {
    /// Creates an empty UI-thread window-command mailbox.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Gives the owning UI thread the current command exactly once.
    pub(crate) fn take(&self) -> Option<WindowCommandRequest<T>> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request.clone())
        }
    }

    /// Reports that the host UI thread applied the command.
    pub(crate) fn complete(&self, request_id: u64) -> bool {
        self.respond(request_id, Ok(()))
    }

    /// Completes the matching request with a safe unavailable result.
    pub(crate) fn fail(&self, request_id: u64) -> bool {
        self.respond(request_id, Err(WindowCommandError::Unavailable))
    }

    /// Hands a command to the UI thread and waits for its bounded answer.
    pub(crate) fn request_within(
        &self,
        value: T,
        timeout: Duration,
    ) -> Result<(), WindowCommandError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let state = &mut *lock(&self.state);
            if state.active.is_some() {
                return Err(WindowCommandError::Busy);
            }
            state.next_id = state.next_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_id;
            state.active = Some(ActiveRequest {
                request: WindowCommandRequest {
                    id: request_id,
                    value,
                },
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        let result = wait_for_response(&response, timeout);
        if result.is_none() {
            // The UI thread never answered. Clear this exact slot so a late
            // response cannot complete a later command and one timeout cannot
            // leave the session permanently busy.
            let state = &mut *lock(&self.state);
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.request.id == request_id)
            {
                state.active = None;
            }
        }
        result.unwrap_or(Err(WindowCommandError::Unavailable))
    }

    fn respond(&self, request_id: u64, result: Result<(), WindowCommandError>) -> bool {
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

fn wait_for_response(
    response: &ResponseSlot,
    timeout: Duration,
) -> Option<Result<(), WindowCommandError>> {
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
