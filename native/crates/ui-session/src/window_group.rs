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
    pending_secondary_closes: Vec<UiWindowId>,
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

#[derive(Clone, Copy)]
enum SecondaryDocumentFormat {
    V1,
    V2,
    V3,
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
                pending_secondary_closes: Vec::new(),
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
                pending_secondary_closes: Vec::new(),
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
    /// Protocol 1.27 reaches only this exact v2 route, preserving the strict
    /// v1 boundary.
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

    /// Replaces one current view's explicit v3 document and publishes it.
    ///
    /// This remains an internal portable route until its matching explicit
    /// protocol operations select the exact version-3 document format.
    pub fn replace_document_v3(
        &self,
        id: &UiWindowId,
        encoded_document: &str,
    ) -> Result<UiWindowSnapshot, UiWindowGroupError> {
        lock(&self.state)
            .windows
            .replace_document_v3(id, encoded_document)
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
        let state = &mut *lock(&self.state);
        state
            .windows
            .close_secondary(id)
            .map_err(map_window_error)?;
        state
            .pending_secondary_closes
            .retain(|pending| pending != id);
        Ok(())
    }

    /// Queues one host-owned close for a current secondary view.
    ///
    /// This records only an opaque logical identity. A native host later takes
    /// the request on its UI thread, resolves it through its private mapping,
    /// and destroys that one native window. The primary `main` view is the
    /// session anchor and is intentionally not closable through this route.
    /// Repeated requests for the same still-open secondary coalesce.
    pub fn request_secondary_close(&self, id: &UiWindowId) -> Result<(), UiWindowGroupError> {
        let state = &mut *lock(&self.state);
        if id.is_primary() || !state.windows.contains(id) {
            return Err(UiWindowGroupError::Unavailable);
        }
        if !state.pending_secondary_closes.contains(id) {
            state.pending_secondary_closes.push(id.clone());
        }
        Ok(())
    }

    /// Takes every coalesced secondary close request for the host UI thread.
    ///
    /// The returned identities remain portable. Resolving them to native
    /// windows is a host-private operation, and actual removal happens only
    /// after that native window is destroyed.
    #[must_use]
    pub fn take_secondary_close_requests(&self) -> Vec<UiWindowId> {
        std::mem::take(&mut lock(&self.state).pending_secondary_closes)
    }

    /// Cancels one worker-to-UI creation handoff during host group shutdown.
    ///
    /// A native group calls this after it has begun closing its views. The
    /// request has no native view yet, so retaining it until the five-second
    /// timeout would strand an authenticated worker after the session is
    /// already ending. The reserved logical identity is rolled back before the
    /// waiter is released, and a UI thread that had not taken the request has
    /// nothing to create.
    ///
    /// Returns `true` only when a request was cancelled. A UI thread that
    /// already took a request may still be between native creation and
    /// [`Self::complete_open`]; that completion returns `false` and requires
    /// the host to destroy the late native view, as documented there.
    pub fn cancel_open_request(&self) -> bool {
        let mut state = lock(&self.state);
        let Some(active) = state.active.take() else {
            return false;
        };
        let _ = state.windows.abort_secondary(active.pending);
        let response = active.response;
        // Hold the group state lock while making the worker outcome visible,
        // matching `complete_open`. A deadline race can therefore observe
        // either an active request or its completed unavailable response, but
        // never a transient empty state.
        let value = &mut *lock(&response.value);
        *value = Some(Err(UiWindowGroupError::Unavailable));
        response.ready.notify_one();
        true
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
        self.open_secondary_inner(
            context,
            encoded_document,
            UI_WINDOW_OPEN_RESPONSE_TIMEOUT,
            SecondaryDocumentFormat::V1,
        )
    }

    /// Sends one validated version-2 secondary-view creation request to the UI
    /// thread. Its lifecycle is identical to [`Self::open_secondary`], but the
    /// document is decoded through the explicit scroll-aware format.
    pub fn open_secondary_v2(
        &self,
        context: T,
        encoded_document: &str,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_inner(
            context,
            encoded_document,
            UI_WINDOW_OPEN_RESPONSE_TIMEOUT,
            SecondaryDocumentFormat::V2,
        )
    }

    /// Sends one validated version-3 secondary-view creation request to the UI
    /// thread. Its lifecycle is identical to [`Self::open_secondary`], but the
    /// document is decoded through the explicit status-aware format.
    pub fn open_secondary_v3(
        &self,
        context: T,
        encoded_document: &str,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_inner(
            context,
            encoded_document,
            UI_WINDOW_OPEN_RESPONSE_TIMEOUT,
            SecondaryDocumentFormat::V3,
        )
    }

    #[cfg(test)]
    fn open_secondary_within(
        &self,
        context: T,
        encoded_document: &str,
        timeout: Duration,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_inner(
            context,
            encoded_document,
            timeout,
            SecondaryDocumentFormat::V1,
        )
    }

    fn open_secondary_inner(
        &self,
        context: T,
        encoded_document: &str,
        timeout: Duration,
        format: SecondaryDocumentFormat,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let mut state = lock(&self.state);
            if state.active.is_some() {
                return Err(UiWindowGroupError::Busy);
            }
            let pending = match format {
                SecondaryDocumentFormat::V1 => state.windows.prepare_secondary(encoded_document),
                SecondaryDocumentFormat::V2 => state.windows.prepare_secondary_v2(encoded_document),
                SecondaryDocumentFormat::V3 => state.windows.prepare_secondary_v3(encoded_document),
            }
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
mod tests;
