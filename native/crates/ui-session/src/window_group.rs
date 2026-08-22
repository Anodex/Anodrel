//! Bounded worker-to-UI-thread coordination for session-owned view creation.
//!
//! The group contains only portable document state and mailboxes. A native host
//! supplies the context carried with a request, creates the actual window on
//! its own UI thread, and must destroy that window if a late completion is no
//! longer accepted.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use anodrel_ui::UiEvent;

use crate::{
    PendingUiWindow, UiApplicationEvent, UiDocumentRevision, UiSessionError, UiWindowId,
    UiWindowInputBatch, UiWindowResources, UiWindowSessionError, UiWindowSessions,
    UiWindowSnapshot,
};

/// Maximum time a worker may wait for its host UI thread to create one view.
///
/// Native creation is quick when the UI thread is healthy. A longer wait would
/// only strand the authenticated pipe worker when its owning UI thread stopped
/// servicing the session.
pub const UI_WINDOW_OPEN_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// A portable group of independently revised views for one UI session.
///
/// `T` is host-owned context for a prospective native window. Application
/// protocol input never constructs it. It might include a composed caption or
/// a private session-lifetime key, but it must never be a native handle.
#[derive(Clone, Debug)]
pub struct UiWindowGroup<T> {
    state: Arc<Mutex<State<T>>>,
}

#[derive(Debug)]
struct State<T> {
    windows: UiWindowSessions,
    next_request_id: u64,
    active: Option<ActiveOpen<T>>,
}

#[derive(Debug)]
struct ActiveOpen<T> {
    id: u64,
    context: T,
    pending: PendingUiWindow,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    value: Mutex<Option<Result<UiWindowId, UiWindowGroupError>>>,
    ready: Condvar,
}

/// A host-owned request to create one already-validated secondary view.
///
/// The request contains neither a native handle nor a way to select another
/// session. The UI thread uses its supplied resources only for the view it is
/// creating and then reports the native outcome by request ID.
#[derive(Clone, Debug)]
pub struct UiWindowOpenRequest<T> {
    id: u64,
    context: T,
    resources: UiWindowResources,
    snapshot: UiWindowSnapshot,
}

impl<T> UiWindowOpenRequest<T> {
    /// Returns the private bridge identity used solely to complete this request.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns host-created context for the prospective native view.
    #[must_use]
    pub const fn context(&self) -> &T {
        &self.context
    }

    /// Returns the resources for the one prospective session-owned view.
    #[must_use]
    pub const fn resources(&self) -> &UiWindowResources {
        &self.resources
    }

    /// Returns the validated initial document paired with its target view.
    #[must_use]
    pub const fn snapshot(&self) -> &UiWindowSnapshot {
        &self.snapshot
    }
}

/// Safe outcomes for a session-window group operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiWindowGroupError {
    /// The supplied document did not meet the existing UI-document contract.
    DocumentRejected(UiSessionError),
    /// Another creation is already waiting for the session's UI thread.
    Busy,
    /// The requested view or host UI-thread operation is unavailable.
    Unavailable,
}

impl std::fmt::Display for UiWindowGroupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DocumentRejected(error) => error.fmt(formatter),
            Self::Busy => formatter.write_str("a UI session window request is already pending"),
            Self::Unavailable => formatter.write_str("the UI session window is unavailable"),
        }
    }
}

impl std::error::Error for UiWindowGroupError {}

impl<T> Default for UiWindowGroup<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> UiWindowGroup<T> {
    /// Creates an empty group containing only its primary `main` view.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                windows: UiWindowSessions::new(),
                next_request_id: 0,
                active: None,
            })),
        }
    }

    /// Creates a group whose primary view uses caller-created mailboxes.
    ///
    /// A host uses this when an authenticated primary view already has its
    /// native resources before it opts into the session-owned group model.
    #[must_use]
    pub fn with_primary_resources(
        document_mailbox: crate::UiDocumentMailbox,
        input_mailbox: crate::UiInputMailbox,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                windows: UiWindowSessions::with_primary_resources(document_mailbox, input_mailbox),
                next_request_id: 0,
                active: None,
            })),
        }
    }

    /// Returns resources for one currently open session-owned view.
    #[must_use]
    pub fn resources(&self, id: &UiWindowId) -> Option<UiWindowResources> {
        lock(&self.state).windows.resources(id)
    }

    /// Returns whether `id` is currently an open view in this group.
    #[must_use]
    pub fn contains(&self, id: &UiWindowId) -> bool {
        lock(&self.state).windows.contains(id)
    }

    /// Replaces one current view's strict v1 document and publishes its snapshot.
    pub fn replace_document(
        &self,
        id: &UiWindowId,
        encoded_document: &str,
    ) -> Result<UiWindowSnapshot, UiWindowGroupError> {
        lock(&self.state)
            .windows
            .replace_document(id, encoded_document)
            .map_err(map_window_error)
    }

    /// Replaces one current view's explicit v2 document and publishes it.
    ///
    /// No released multi-window protocol operation reaches this method yet.
    /// It exists so a future v2 view operation can preserve the v1 boundary.
    pub fn replace_document_v2(
        &self,
        id: &UiWindowId,
        encoded_document: &str,
    ) -> Result<UiWindowSnapshot, UiWindowGroupError> {
        lock(&self.state)
            .windows
            .replace_document_v2(id, encoded_document)
            .map_err(map_window_error)
    }

    /// Validates one view-local semantic candidate against its own document.
    pub fn accept_event(
        &self,
        id: &UiWindowId,
        revision: UiDocumentRevision,
        event: UiEvent,
    ) -> Result<UiApplicationEvent, UiWindowSessionError> {
        lock(&self.state).windows.accept_event(id, revision, event)
    }

    /// Drains the bounded semantic-input queue for each currently open view.
    ///
    /// The result is for an authenticated session core, not a protocol
    /// enumeration surface. It contains no native handle or desktop state.
    #[must_use]
    pub fn drain_input_batches(&self) -> Vec<UiWindowInputBatch> {
        lock(&self.state).windows.drain_input_batches()
    }

    /// Drains the bounded semantic-input queue for one known logical view.
    ///
    /// This supports primary-only compatibility paths without consuming a
    /// secondary view's local input. An unavailable view returns only the
    /// existing safe group category.
    pub fn drain_input_batch(
        &self,
        id: &UiWindowId,
    ) -> Result<crate::UiInputBatch, UiWindowGroupError> {
        lock(&self.state)
            .windows
            .drain_input_batch(id)
            .map_err(map_window_error)
    }

    /// Removes one secondary view after its native window is gone.
    pub fn close_secondary(&self, id: &UiWindowId) -> Result<(), UiWindowGroupError> {
        lock(&self.state)
            .windows
            .close_secondary(id)
            .map_err(map_window_error)
    }
}

impl<T: Clone> UiWindowGroup<T> {
    /// Sends one validated secondary-view creation request to the UI thread.
    ///
    /// A returned identity means the UI thread created and registered the
    /// native view, then committed the portable view. A native caller whose
    /// [`Self::complete_open`] returns `false` must discard any just-created
    /// window, because the worker timed out and the group rolled the pending
    /// view back.
    pub fn open_secondary(
        &self,
        context: T,
        encoded_document: &str,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_within(context, encoded_document, UI_WINDOW_OPEN_RESPONSE_TIMEOUT)
    }

    fn open_secondary_within(
        &self,
        context: T,
        encoded_document: &str,
        timeout: Duration,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_inner(context, encoded_document, timeout)
    }

    fn open_secondary_inner(
        &self,
        context: T,
        encoded_document: &str,
        timeout: Duration,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let mut state = lock(&self.state);
            if state.active.is_some() {
                return Err(UiWindowGroupError::Busy);
            }
            let pending = state
                .windows
                .prepare_secondary(encoded_document)
                .map_err(map_window_error)?;
            state.next_request_id = state.next_request_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_request_id;
            state.active = Some(ActiveOpen {
                id: request_id,
                context,
                pending,
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        let result = wait_for_response(&response, timeout);
        if result.is_none() {
            let mut state = lock(&self.state);
            let Some(active) = state.active.take() else {
                // `complete_open` publishes its response before it releases
                // this state lock. It won the deadline race, so return that
                // already-committed outcome instead of reporting a false
                // timeout to the caller.
                drop(state);
                return take_completed_response(&response)
                    .unwrap_or(Err(UiWindowGroupError::Unavailable));
            };
            if active.id != request_id {
                state.active = Some(active);
                return Err(UiWindowGroupError::Unavailable);
            }
            let _ = state.windows.abort_secondary(active.pending);
            return Err(UiWindowGroupError::Unavailable);
        }
        result.expect("response exists when the wait did not time out")
    }

    /// Takes one native-view creation request exactly once on the UI thread.
    #[must_use]
    pub fn take_open_request(&self) -> Option<UiWindowOpenRequest<T>> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            return None;
        }
        active.taken = true;
        Some(UiWindowOpenRequest {
            id: active.id,
            context: active.context.clone(),
            resources: active.pending.resources(),
            snapshot: UiWindowSnapshot::new(
                active.pending.id().clone(),
                active.pending.snapshot().clone(),
            ),
        })
    }

    /// Completes one native creation request.
    ///
    /// `true` commits and publishes the prepared view. `false` aborts it. The
    /// boolean return is false for an expired or unknown request, telling a
    /// native host to destroy a late-created window rather than leave it bound
    /// to a rolled-back session view.
    pub fn complete_open(&self, request_id: u64, created: bool) -> bool {
        {
            let mut state = lock(&self.state);
            let Some(active) = state.active.as_ref() else {
                return false;
            };
            if active.id != request_id || !active.taken {
                return false;
            }
            let active = state.active.take().expect("active request was present");
            let response = Arc::clone(&active.response);
            let result = if created {
                match state.windows.commit_secondary(active.pending) {
                    Ok(snapshot) => Ok(snapshot.id().clone()),
                    Err(_) => Err(UiWindowGroupError::Unavailable),
                }
            } else {
                let _ = state.windows.abort_secondary(active.pending);
                Err(UiWindowGroupError::Unavailable)
            };
            // Keep the state lock while publishing the outcome. A timed-out
            // worker that observes no active request can then only observe a
            // completed response, never a transient empty slot.
            let value = &mut *lock(&response.value);
            *value = Some(result);
            response.ready.notify_one();
        }
        true
    }

    /// Fails one native creation request without exposing the native cause.
    pub fn fail_open(&self, request_id: u64) -> bool {
        self.complete_open(request_id, false)
    }
}

fn wait_for_response(
    response: &ResponseSlot,
    timeout: Duration,
) -> Option<Result<UiWindowId, UiWindowGroupError>> {
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

fn take_completed_response(
    response: &ResponseSlot,
) -> Option<Result<UiWindowId, UiWindowGroupError>> {
    lock(&response.value).take()
}

fn map_window_error(error: UiWindowSessionError) -> UiWindowGroupError {
    match error {
        UiWindowSessionError::DocumentRejected(error) => {
            UiWindowGroupError::DocumentRejected(error)
        }
        UiWindowSessionError::OpenBusy => UiWindowGroupError::Busy,
        UiWindowSessionError::EventRejected(_)
        | UiWindowSessionError::OpenLimitReached
        | UiWindowSessionError::IdentityExhausted
        | UiWindowSessionError::WindowUnavailable
        | UiWindowSessionError::PrimaryCannotClose
        | UiWindowSessionError::PendingWindowInvalid => UiWindowGroupError::Unavailable,
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

    use anodrel_ui::{ElementId, UiEvent};

    use super::{UiWindowGroup, UiWindowGroupError};
    use crate::{UiDocumentMailbox, UiInputCandidate, UiInputMailbox, UiWindowId};

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Secondary","fontSize":16,"tone":"primary"}}"#;

    fn take_pending(
        group: &UiWindowGroup<&'static str>,
    ) -> super::UiWindowOpenRequest<&'static str> {
        loop {
            if let Some(request) = group.take_open_request() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn commits_only_after_the_ui_thread_created_the_view() {
        let group = UiWindowGroup::new();
        let worker = group.clone();
        let (sent, received) = mpsc::channel();
        let waiting = thread::spawn(move || sent.send(worker.open_secondary("caption", DOCUMENT)));

        let request = take_pending(&group);
        assert_eq!(request.context(), &"caption");
        assert_eq!(request.resources().id().to_protocol_string(), "window-1");
        assert_eq!(request.snapshot().snapshot().revision().value(), 1);
        assert!(request.resources().document_mailbox().take().is_none());
        assert!(group.complete_open(request.id(), true));

        let id = received
            .recv()
            .expect("worker returns a response")
            .expect("view opens");
        waiting
            .join()
            .expect("worker does not panic")
            .expect("worker submits its response");
        assert_eq!(id.to_protocol_string(), "window-1");
        assert!(group.contains(&id));
        assert_eq!(
            request
                .resources()
                .document_mailbox()
                .take()
                .expect("commit publishes the initial document")
                .revision()
                .value(),
            1
        );
    }

    #[test]
    fn failed_native_creation_aborts_the_unissued_identity() {
        let group = UiWindowGroup::new();
        let worker = group.clone();
        let (sent, received) = mpsc::channel();
        let waiting = thread::spawn(move || sent.send(worker.open_secondary("caption", DOCUMENT)));
        let request = take_pending(&group);
        assert!(group.fail_open(request.id()));
        assert_eq!(
            received.recv().expect("worker returns a response"),
            Err(UiWindowGroupError::Unavailable)
        );
        waiting
            .join()
            .expect("worker does not panic")
            .expect("worker submits its response");
        assert!(!group.contains(request.resources().id()));

        let worker = group.clone();
        let (sent, received) = mpsc::channel();
        let waiting = thread::spawn(move || sent.send(worker.open_secondary("retry", DOCUMENT)));
        let retry = take_pending(&group);
        assert_eq!(retry.resources().id().to_protocol_string(), "window-1");
        assert!(group.complete_open(retry.id(), true));
        assert_eq!(
            received
                .recv()
                .expect("retry returns a response")
                .expect("retry opens")
                .to_protocol_string(),
            "window-1"
        );
        waiting
            .join()
            .expect("retry worker does not panic")
            .expect("retry worker submits its response");
    }

    #[test]
    fn keeps_one_native_creation_in_flight_and_rejects_late_completion() {
        let group = UiWindowGroup::new();
        let worker = group.clone();
        let waiting = thread::spawn(move || {
            worker.open_secondary_within("caption", DOCUMENT, Duration::from_millis(20))
        });
        let request = take_pending(&group);
        assert_eq!(
            group.open_secondary("second", DOCUMENT),
            Err(UiWindowGroupError::Busy)
        );
        assert_eq!(
            waiting.join().expect("worker does not panic"),
            Err(UiWindowGroupError::Unavailable)
        );
        assert!(
            !group.complete_open(request.id(), true),
            "a host must destroy a window created after its worker timed out"
        );
        assert!(!group.contains(&UiWindowId::parse("window-1").expect("ID parses")));
    }

    #[test]
    fn rejects_invalid_documents_before_a_ui_thread_request_exists() {
        let group = UiWindowGroup::<&'static str>::new();
        assert!(matches!(
            group.open_secondary("caption", "not a document"),
            Err(UiWindowGroupError::DocumentRejected(_))
        ));
        assert!(group.take_open_request().is_none());
    }

    #[test]
    fn binds_the_existing_primary_mailboxes_into_the_group_without_copying_state() {
        let document_mailbox = UiDocumentMailbox::new();
        let input_mailbox = UiInputMailbox::new();
        let group = UiWindowGroup::<()>::with_primary_resources(
            document_mailbox.clone(),
            input_mailbox.clone(),
        );
        let primary = UiWindowId::primary();

        let snapshot = group
            .replace_document(&primary, DOCUMENT)
            .expect("the primary document validates");
        assert_eq!(
            document_mailbox
                .take()
                .expect("the caller-owned mailbox receives the primary snapshot")
                .revision(),
            snapshot.snapshot().revision()
        );

        input_mailbox.push(UiInputCandidate::new(
            snapshot.snapshot().revision(),
            UiEvent::ActionInvoked(ElementId::new("root").expect("fixed action ID is valid")),
        ));
        let batches = group.drain_input_batches();
        assert_eq!(batches.len(), 1);
        let (id, batch) = batches
            .into_iter()
            .next()
            .expect("the primary batch is present")
            .into_parts();
        assert_eq!(id, primary);
        assert_eq!(batch.dropped(), 0);
        assert_eq!(batch.into_candidates().len(), 1);
    }
}
