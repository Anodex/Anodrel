//! One bounded, host-UI-thread bridge for reading field values.
//!
//! The text a person typed lives with the window that owns it, on the UI
//! thread. A protocol worker never touches that state directly, so it hands a
//! request to this mailbox and the owning UI thread answers with a snapshot.
//!
//! The same shape as the notification and window-title bridges — one pending
//! request, taken exactly once, with a timeout that frees the session — with
//! one difference: the response carries a value rather than only a result.
//! That is the whole reason this exists as its own type.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crate::{UiFieldReadError, UiFieldReader, UiFieldSnapshot};

/// Maximum time a protocol worker may wait for its host UI thread to respond.
///
/// Matches the other UI-thread bridges. Copying a handful of short strings is
/// not slow, so a worker blocked longer than this is stuck rather than patient.
pub const UI_FIELD_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// A pending read, taken exactly once by the host UI thread.
///
/// Carries an identity and nothing else. There is deliberately no field
/// selector on this request: the answer is the whole surface, so there is
/// nothing for a caller to narrow. See `docs/UI_FIELDS.md`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiFieldRequest {
    id: u64,
}

impl UiFieldRequest {
    /// Returns the identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(self) -> u64 {
        self.id
    }
}

/// A one-request bridge from a protocol worker to one host UI thread.
#[derive(Clone, Debug, Default)]
pub struct UiFieldMailbox {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: Option<ActiveRequest>,
}

#[derive(Debug)]
struct ActiveRequest {
    request: UiFieldRequest,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    value: Mutex<Option<Result<UiFieldSnapshot, UiFieldReadError>>>,
    ready: Condvar,
}

impl UiFieldMailbox {
    /// Creates an empty UI-thread field-read mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the current read once so the owning UI thread can answer it.
    #[must_use]
    pub fn take(&self) -> Option<UiFieldRequest> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request)
        }
    }

    /// Answers the matching read with the surface's current values.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn complete(&self, request_id: u64, snapshot: UiFieldSnapshot) -> bool {
        self.respond(request_id, Ok(snapshot))
    }

    /// Answers the matching read with a safe unavailable result.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn fail(&self, request_id: u64) -> bool {
        self.respond(request_id, Err(UiFieldReadError::Unavailable))
    }

    fn read_within(&self, timeout: Duration) -> Result<UiFieldSnapshot, UiFieldReadError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let state = &mut *lock(&self.state);
            if state.active.is_some() {
                // A second concurrent read reports the one unavailable code
                // rather than a distinct busy one. Distinguishing them would
                // let a caller detect that another read was in flight, and this
                // operation reports nothing about host state.
                return Err(UiFieldReadError::Unavailable);
            }
            state.next_id = state.next_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_id;
            state.active = Some(ActiveRequest {
                request: UiFieldRequest { id: request_id },
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        let result = wait_for_response(&response, timeout);
        if result.is_none() {
            // The UI thread never answered. Clear the slot so this session is
            // not left permanently unable to read by one stuck request.
            let state = &mut *lock(&self.state);
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.request.id == request_id)
            {
                state.active = None;
            }
        }
        result.unwrap_or(Err(UiFieldReadError::Unavailable))
    }

    fn respond(&self, request_id: u64, result: Result<UiFieldSnapshot, UiFieldReadError>) -> bool {
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

impl UiFieldReader for UiFieldMailbox {
    fn read(&self) -> Result<UiFieldSnapshot, UiFieldReadError> {
        self.read_within(UI_FIELD_RESPONSE_TIMEOUT)
    }
}

fn wait_for_response(
    response: &ResponseSlot,
    timeout: Duration,
) -> Option<Result<UiFieldSnapshot, UiFieldReadError>> {
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
    use std::{thread, time::Duration};

    use super::{UiFieldMailbox, UiFieldRequest};
    use crate::{UiFieldReadError, UiFieldReader, UiFieldSnapshot};
    use anodrel_ui::{Axis, ElementId, Field, Insets, Stack, UiDocument, UiFieldStates, UiNode};

    fn snapshot(value: &str) -> UiFieldSnapshot {
        let id = ElementId::new("name").expect("test ID is valid");
        let document = UiDocument::new(UiNode::Stack(
            Stack::new(
                ElementId::new("root").expect("test ID is valid"),
                Axis::Vertical,
                Insets::zero(),
                0,
                vec![UiNode::Field(
                    Field::new(id, "Label", value, 64, 14, true).expect("test field is valid"),
                )],
            )
            .expect("test stack is valid"),
        ))
        .expect("test document is valid");
        let mut states = UiFieldStates::new();
        states.reseed(&document);
        UiFieldSnapshot::from_states(&states).expect("the snapshot builds")
    }

    /// Spins until the UI-thread side can take the pending read.
    fn take_pending(mailbox: &UiFieldMailbox) -> UiFieldRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn a_read_crosses_to_the_ui_thread_and_returns_its_values() {
        let mailbox = UiFieldMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read());

        let request = take_pending(&mailbox);
        // Handed over exactly once.
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(request.id(), snapshot("Ada")));

        let received = waiting
            .join()
            .expect("the worker did not panic")
            .expect("the read succeeded");
        assert_eq!(received.fields().len(), 1);
        assert_eq!(received.fields()[0].value(), "Ada");
    }

    #[test]
    fn a_request_carries_no_selector_for_the_ui_thread_to_honour() {
        // The UI thread receives an identity and nothing else, so there is no
        // place for a field name to travel even if a caller sent one.
        let mailbox = UiFieldMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read());
        let request = take_pending(&mailbox);
        let debug = format!("{request:?}");
        assert!(debug.contains("id"));
        for absent in ["field", "selector", "name", "filter"] {
            assert!(!debug.contains(absent), "{absent} reached a read request");
        }
        assert!(mailbox.fail(request.id()));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(UiFieldReadError::Unavailable)
        );
    }

    #[test]
    fn a_second_concurrent_read_reports_the_same_unavailable_code() {
        // Not a distinct "busy": that would let a caller detect another read in
        // flight, and this operation reports nothing about host state.
        let mailbox = UiFieldMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read());

        let request = take_pending(&mailbox);
        assert_eq!(mailbox.read(), Err(UiFieldReadError::Unavailable));
        assert!(mailbox.complete(request.id(), snapshot("Ada")));
        assert!(waiting.join().expect("the worker did not panic").is_ok());
    }

    #[test]
    fn a_read_the_ui_thread_never_answers_frees_the_session_again() {
        let mailbox = UiFieldMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read_within(Duration::from_millis(20)));

        let _abandoned = take_pending(&mailbox);
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(UiFieldReadError::Unavailable)
        );

        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.read());
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id(), snapshot("Grace")));
        assert!(next.join().expect("the worker did not panic").is_ok());
    }

    #[test]
    fn ignores_a_completion_for_an_untaken_or_unknown_request() {
        let mailbox = UiFieldMailbox::new();
        assert!(!mailbox.complete(1, snapshot("Ada")));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.read());
        while mailbox.take().is_none() {
            thread::yield_now();
        }
        assert!(!mailbox.complete(9_999, snapshot("Ada")));
        assert!(mailbox.fail(1));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(UiFieldReadError::Unavailable)
        );
    }
}
