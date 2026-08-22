//! Bounded, logical multi-view state for one authenticated UI session.
//!
//! This module owns no native windows, transport endpoint, application identity,
//! or permission decision. It is the portable state beneath the future
//! session-owned multi-window host contract in `docs/MULTI_WINDOW.md`.

use std::{collections::BTreeMap, fmt, num::NonZeroU16};

use anodrel_ui::UiEvent;

use crate::{
    UiApplicationEvent, UiDocumentMailbox, UiDocumentRevision, UiDocumentSession,
    UiDocumentSnapshot, UiInputBatch, UiInputMailbox, UiSessionError, UiWindowId,
};

/// The maximum number of concurrently open logical views in one UI session.
///
/// This includes the primary `main` view and therefore permits at most three
/// secondary views.
pub const MAX_SESSION_WINDOWS: usize = 4;

/// Host-facing resources for one session-owned logical view.
///
/// A native host may use these portable mailboxes to connect one already-known
/// view to its UI thread. Holding these values never grants a native handle or
/// authority to create, enumerate, or target a desktop window.
#[derive(Clone, Debug)]
pub struct UiWindowResources {
    id: UiWindowId,
    document_mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
}

/// One view's bounded native semantic-input batch.
///
/// This is a host-internal grouping value. It does not provide application
/// window enumeration: a protocol operation decides separately whether and how
/// it exposes an accepted event.
#[derive(Debug)]
pub struct UiWindowInputBatch {
    id: UiWindowId,
    batch: UiInputBatch,
}

impl UiWindowInputBatch {
    /// Returns the session-owned view that produced this input batch.
    #[must_use]
    pub const fn id(&self) -> &UiWindowId {
        &self.id
    }

    /// Splits the batch into its private logical identity and input values.
    #[must_use]
    pub fn into_parts(self) -> (UiWindowId, UiInputBatch) {
        (self.id, self.batch)
    }
}

impl UiWindowResources {
    /// Returns the session-scoped logical identity for these resources.
    #[must_use]
    pub const fn id(&self) -> &UiWindowId {
        &self.id
    }

    /// Returns this view's newest-snapshot delivery mailbox.
    #[must_use]
    pub fn document_mailbox(&self) -> UiDocumentMailbox {
        self.document_mailbox.clone()
    }

    /// Returns this view's bounded semantic-input mailbox.
    #[must_use]
    pub fn input_mailbox(&self) -> UiInputMailbox {
        self.input_mailbox.clone()
    }
}

/// One accepted document snapshot tied to its session-owned logical view.
#[derive(Clone, Debug)]
pub struct UiWindowSnapshot {
    id: UiWindowId,
    snapshot: UiDocumentSnapshot,
}

impl UiWindowSnapshot {
    /// Builds a view-scoped snapshot inside the UI-session crate.
    ///
    /// External callers can inspect a snapshot but cannot pair an arbitrary
    /// document with a logical view identity.
    #[must_use]
    pub(crate) fn new(id: UiWindowId, snapshot: UiDocumentSnapshot) -> Self {
        Self { id, snapshot }
    }

    /// Returns the view whose document revision this snapshot carries.
    #[must_use]
    pub const fn id(&self) -> &UiWindowId {
        &self.id
    }

    /// Returns the immutable revision-bound document snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &UiDocumentSnapshot {
        &self.snapshot
    }
}

/// A validated secondary view awaiting the host's native creation outcome.
///
/// The pending value is deliberately not part of its containing session set.
/// A host calls [`UiWindowSessions::abort_secondary`] after native creation
/// fails, leaving no routable identity or mailbox behind. It commits the value
/// only after it has created and registered the corresponding native view.
#[derive(Debug)]
pub struct PendingUiWindow {
    state: UiWindowState,
    snapshot: UiDocumentSnapshot,
}

impl PendingUiWindow {
    /// Returns the reserved identity that will be issued only on commit.
    #[must_use]
    pub const fn id(&self) -> &UiWindowId {
        &self.state.id
    }

    /// Returns the resources a host needs while creating the native view.
    #[must_use]
    pub fn resources(&self) -> UiWindowResources {
        self.state.resources()
    }

    /// Returns the fully validated initial document for the prospective view.
    #[must_use]
    pub const fn snapshot(&self) -> &UiDocumentSnapshot {
        &self.snapshot
    }
}

/// Stable safe categories for logical multi-view session operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiWindowSessionError {
    /// A document failed the existing exact UI-document contract.
    DocumentRejected(UiSessionError),
    /// A semantic action failed the existing revision-bound document checks.
    EventRejected(UiSessionError),
    /// The session already has the fixed maximum number of open views.
    OpenLimitReached,
    /// A host has one native secondary-view creation request in flight.
    OpenBusy,
    /// No more non-reusable secondary identities can be issued this session.
    IdentityExhausted,
    /// The named view does not currently belong to this session.
    WindowUnavailable,
    /// Closing the primary view is reserved for the group-wide session-close path.
    PrimaryCannotClose,
    /// A host tried to commit a pending view after the session state changed.
    PendingWindowInvalid,
}

impl fmt::Display for UiWindowSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::DocumentRejected(error) => return error.fmt(formatter),
            Self::EventRejected(error) => return error.fmt(formatter),
            Self::OpenLimitReached => "UI session has reached its view limit",
            Self::OpenBusy => "UI session already has a pending secondary view",
            Self::IdentityExhausted => "UI session view identity space is exhausted",
            Self::WindowUnavailable => "UI session view is unavailable",
            Self::PrimaryCannotClose => "the primary UI session view cannot close independently",
            Self::PendingWindowInvalid => "pending UI session view is no longer valid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UiWindowSessionError {}

/// Portable state for the independently revised views in one UI session.
///
/// This is deliberately not a native window registry. It cannot create or
/// close an OS surface; a host uses the prepare/commit split around its own
/// UI-thread work and calls [`Self::abort_secondary`] on failure.
#[derive(Debug)]
pub struct UiWindowSessions {
    windows: BTreeMap<UiWindowId, UiWindowState>,
    next_secondary: Option<NonZeroU16>,
    pending_secondary: Option<NonZeroU16>,
}

impl Default for UiWindowSessions {
    fn default() -> Self {
        Self::new()
    }
}

impl UiWindowSessions {
    /// Creates a group containing exactly the empty primary `main` view.
    #[must_use]
    pub fn new() -> Self {
        Self::with_primary_resources(UiDocumentMailbox::new(), UiInputMailbox::new())
    }

    /// Creates a group whose primary view uses caller-created mailboxes.
    ///
    /// This lets a host bind its already-known primary native view and its
    /// authenticated session core to one portable group without cloning
    /// document or input state into a parallel session.
    #[must_use]
    pub fn with_primary_resources(
        document_mailbox: UiDocumentMailbox,
        input_mailbox: UiInputMailbox,
    ) -> Self {
        let primary =
            UiWindowState::with_resources(UiWindowId::Primary, document_mailbox, input_mailbox);
        Self {
            windows: BTreeMap::from([(UiWindowId::Primary, primary)]),
            next_secondary: NonZeroU16::new(1),
            pending_secondary: None,
        }
    }

    /// Returns the number of currently open logical views.
    #[must_use]
    pub fn open_count(&self) -> usize {
        self.windows.len()
    }

    /// Returns whether this session currently owns `id`.
    #[must_use]
    pub fn contains(&self, id: &UiWindowId) -> bool {
        self.windows.contains_key(id)
    }

    /// Returns a clone of one current view's host-facing resources.
    #[must_use]
    pub fn resources(&self, id: &UiWindowId) -> Option<UiWindowResources> {
        self.windows.get(id).map(UiWindowState::resources)
    }

    /// Validates an initial v1 document and reserves the next secondary view.
    ///
    /// The returned identity and its resources do not become reachable through
    /// this group until [`Self::commit_secondary`] succeeds.
    pub fn prepare_secondary(
        &mut self,
        encoded_document: &str,
    ) -> Result<PendingUiWindow, UiWindowSessionError> {
        if self.windows.len() == MAX_SESSION_WINDOWS {
            return Err(UiWindowSessionError::OpenLimitReached);
        }
        if self.pending_secondary.is_some() {
            return Err(UiWindowSessionError::OpenBusy);
        }
        let number = self
            .next_secondary
            .ok_or(UiWindowSessionError::IdentityExhausted)?;
        let mut state = UiWindowState::new(UiWindowId::Secondary(number));
        let revision = state
            .document_session
            .replace_document(encoded_document)
            .map_err(UiWindowSessionError::DocumentRejected)?;
        let snapshot = state
            .document_session
            .snapshot()
            .expect("accepted UI document has a snapshot");
        debug_assert_eq!(snapshot.revision(), revision);
        self.pending_secondary = Some(number);
        Ok(PendingUiWindow { state, snapshot })
    }

    /// Rolls back one pending secondary after the host could not create it.
    ///
    /// The identity was never committed or exposed through this group, so the
    /// next successful creation may use the same number. Hosts must call this
    /// before answering a failed native-creation request.
    pub fn abort_secondary(
        &mut self,
        pending: PendingUiWindow,
    ) -> Result<(), UiWindowSessionError> {
        let UiWindowId::Secondary(number) = pending.state.id else {
            return Err(UiWindowSessionError::PendingWindowInvalid);
        };
        if self.pending_secondary != Some(number) {
            return Err(UiWindowSessionError::PendingWindowInvalid);
        }
        self.pending_secondary = None;
        Ok(())
    }

    /// Commits a secondary view after the host registered its native surface.
    ///
    /// The accepted initial snapshot is published only now. A native view that
    /// polls before commit therefore sees no document rather than a document
    /// for an identity the host has not issued yet.
    pub fn commit_secondary(
        &mut self,
        pending: PendingUiWindow,
    ) -> Result<UiWindowSnapshot, UiWindowSessionError> {
        let expected = self
            .next_secondary
            .ok_or(UiWindowSessionError::IdentityExhausted)?;
        if self.pending_secondary != Some(expected)
            || pending.state.id != UiWindowId::Secondary(expected)
        {
            return Err(UiWindowSessionError::PendingWindowInvalid);
        }
        let id = pending.state.id.clone();
        let snapshot = pending.snapshot;
        let mailbox = pending.state.document_mailbox.clone();
        if self.windows.insert(id.clone(), pending.state).is_some() {
            return Err(UiWindowSessionError::PendingWindowInvalid);
        }
        self.pending_secondary = None;
        self.next_secondary = expected.checked_add(1);
        mailbox.publish(snapshot.clone());
        Ok(UiWindowSnapshot { id, snapshot })
    }

    /// Replaces one currently open view's v1 document and publishes its snapshot.
    pub fn replace_document(
        &mut self,
        id: &UiWindowId,
        encoded_document: &str,
    ) -> Result<UiWindowSnapshot, UiWindowSessionError> {
        let state = self
            .windows
            .get_mut(id)
            .ok_or(UiWindowSessionError::WindowUnavailable)?;
        state
            .document_session
            .replace_document(encoded_document)
            .map_err(UiWindowSessionError::DocumentRejected)?;
        let snapshot = state
            .document_session
            .snapshot()
            .expect("accepted UI document has a snapshot");
        state.document_mailbox.publish(snapshot.clone());
        Ok(UiWindowSnapshot {
            id: id.clone(),
            snapshot,
        })
    }

    /// Replaces one currently open view's explicit v2 document and publishes it.
    ///
    /// This portable method exists for a later exact protocol operation. It
    /// does not widen the reserved v1 multi-window request contract.
    pub fn replace_document_v2(
        &mut self,
        id: &UiWindowId,
        encoded_document: &str,
    ) -> Result<UiWindowSnapshot, UiWindowSessionError> {
        let state = self
            .windows
            .get_mut(id)
            .ok_or(UiWindowSessionError::WindowUnavailable)?;
        state
            .document_session
            .replace_document_v2(encoded_document)
            .map_err(UiWindowSessionError::DocumentRejected)?;
        let snapshot = state
            .document_session
            .snapshot()
            .expect("accepted UI document has a snapshot");
        state.document_mailbox.publish(snapshot.clone());
        Ok(UiWindowSnapshot {
            id: id.clone(),
            snapshot,
        })
    }

    /// Validates one view-local semantic action against its own document state.
    pub fn accept_event(
        &self,
        id: &UiWindowId,
        revision: UiDocumentRevision,
        event: UiEvent,
    ) -> Result<UiApplicationEvent, UiWindowSessionError> {
        self.windows
            .get(id)
            .ok_or(UiWindowSessionError::WindowUnavailable)?
            .document_session
            .accept_event(revision, event)
            .map_err(UiWindowSessionError::EventRejected)
    }

    /// Drains each open view's bounded native input queue.
    ///
    /// Batches are returned in logical-identity order for deterministic host
    /// processing. Their contents retain only per-view input order; no caller
    /// may infer a global desktop order across different views.
    #[must_use]
    pub fn drain_input_batches(&self) -> Vec<UiWindowInputBatch> {
        self.windows
            .iter()
            .map(|(id, state)| UiWindowInputBatch {
                id: id.clone(),
                batch: state.input_mailbox.drain(),
            })
            .collect()
    }

    /// Drains the bounded native input queue for one known session-owned view.
    ///
    /// A caller that preserves a legacy primary-only operation uses this method
    /// rather than draining other views' queues as a side effect.
    pub fn drain_input_batch(&self, id: &UiWindowId) -> Result<UiInputBatch, UiWindowSessionError> {
        self.windows
            .get(id)
            .map(|state| state.input_mailbox.drain())
            .ok_or(UiWindowSessionError::WindowUnavailable)
    }

    /// Removes one secondary view after its host-owned native view has closed.
    ///
    /// A closed identity remains unavailable even when capacity later permits a
    /// different secondary view. The primary can leave only through a
    /// group-wide session shutdown.
    pub fn close_secondary(&mut self, id: &UiWindowId) -> Result<(), UiWindowSessionError> {
        if id.is_primary() {
            return Err(UiWindowSessionError::PrimaryCannotClose);
        }
        self.windows
            .remove(id)
            .map(|_| ())
            .ok_or(UiWindowSessionError::WindowUnavailable)
    }
}

#[derive(Debug)]
struct UiWindowState {
    id: UiWindowId,
    document_session: UiDocumentSession,
    document_mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
}

impl UiWindowState {
    fn new(id: UiWindowId) -> Self {
        Self::with_resources(id, UiDocumentMailbox::new(), UiInputMailbox::new())
    }

    fn with_resources(
        id: UiWindowId,
        document_mailbox: UiDocumentMailbox,
        input_mailbox: UiInputMailbox,
    ) -> Self {
        Self {
            id,
            document_session: UiDocumentSession::new(),
            document_mailbox,
            input_mailbox,
        }
    }

    fn resources(&self) -> UiWindowResources {
        UiWindowResources {
            id: self.id.clone(),
            document_mailbox: self.document_mailbox.clone(),
            input_mailbox: self.input_mailbox.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_ui::{ElementId, UiEvent};

    use super::{MAX_SESSION_WINDOWS, UiWindowId, UiWindowSessionError, UiWindowSessions};
    use crate::UiWindowIdError;

    const ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}"#;
    const OTHER_ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"other","kind":"action","label":"Other","fontSize":16,"enabled":true,"tone":"accent"}}"#;

    fn continue_event() -> UiEvent {
        UiEvent::ActionInvoked(ElementId::new("root").expect("fixed action ID is valid"))
    }

    #[test]
    fn recognizes_only_canonical_session_window_identities() {
        assert_eq!(
            UiWindowId::parse("main").expect("primary ID parses"),
            UiWindowId::primary()
        );
        assert_eq!(
            UiWindowId::parse("window-17")
                .expect("secondary ID parses")
                .to_protocol_string(),
            "window-17"
        );
        for invalid in [
            "",
            "Main",
            "window-0",
            "window-01",
            "window--1",
            "window-65536",
        ] {
            assert_eq!(UiWindowId::parse(invalid), Err(UiWindowIdError::Invalid));
        }
    }

    #[test]
    fn commits_a_valid_secondary_only_after_host_creation_succeeds() {
        let mut windows = UiWindowSessions::new();
        let pending = windows
            .prepare_secondary(ACTION_DOCUMENT)
            .expect("initial document validates");
        let resources = pending.resources();
        assert_eq!(pending.id().to_protocol_string(), "window-1");
        assert_eq!(pending.snapshot().revision().value(), 1);
        assert_eq!(windows.open_count(), 1);
        assert!(!windows.contains(pending.id()));
        assert!(resources.document_mailbox().take().is_none());

        let committed = windows
            .commit_secondary(pending)
            .expect("host registration commits the view");
        assert_eq!(windows.open_count(), 2);
        assert!(windows.contains(committed.id()));
        assert_eq!(committed.snapshot().revision().value(), 1);
        assert_eq!(
            resources
                .document_mailbox()
                .take()
                .expect("commit publishes the first snapshot")
                .revision()
                .value(),
            1
        );
    }

    #[test]
    fn keeps_revisions_and_semantic_actions_independent_per_view() {
        let mut windows = UiWindowSessions::new();
        let first_pending = windows
            .prepare_secondary(ACTION_DOCUMENT)
            .expect("first document validates");
        let first = windows
            .commit_secondary(first_pending)
            .expect("first view commits")
            .id()
            .clone();
        let second_pending = windows
            .prepare_secondary(OTHER_ACTION_DOCUMENT)
            .expect("second document validates");
        let second = windows
            .commit_secondary(second_pending)
            .expect("second view commits")
            .id()
            .clone();

        let replacement = windows
            .replace_document(&first, ACTION_DOCUMENT)
            .expect("first view updates");
        assert_eq!(replacement.snapshot().revision().value(), 2);
        assert_eq!(
            windows
                .accept_event(&first, replacement.snapshot().revision(), continue_event())
                .expect("first current action is accepted")
                .revision()
                .value(),
            2
        );
        let second_replacement = windows
            .replace_document(&second, OTHER_ACTION_DOCUMENT)
            .expect("second view updates independently");
        assert_eq!(
            second_replacement.snapshot().revision(),
            replacement.snapshot().revision(),
            "equal numeric revisions remain scoped to different views"
        );
        assert_eq!(
            windows.accept_event(
                &second,
                second_replacement.snapshot().revision(),
                continue_event()
            ),
            Err(UiWindowSessionError::EventRejected(
                super::UiSessionError::ActionUnavailable
            ))
        );
    }

    #[test]
    fn failed_open_never_consumes_an_identity_or_mutates_the_group() {
        let mut windows = UiWindowSessions::new();
        assert!(matches!(
            windows.prepare_secondary("not a document"),
            Err(UiWindowSessionError::DocumentRejected(_))
        ));
        let pending = windows
            .prepare_secondary(ACTION_DOCUMENT)
            .expect("a valid first request still reserves the first identity");
        assert_eq!(pending.id().to_protocol_string(), "window-1");
        assert_eq!(windows.open_count(), 1);
    }

    #[test]
    fn enforces_a_small_open_set_and_never_reuses_a_closed_identity() {
        let mut windows = UiWindowSessions::new();
        let mut ids = Vec::new();
        for expected in 1..MAX_SESSION_WINDOWS {
            let pending = windows
                .prepare_secondary(ACTION_DOCUMENT)
                .expect("capacity remains");
            assert_eq!(
                pending.id().to_protocol_string(),
                format!("window-{expected}")
            );
            ids.push(
                windows
                    .commit_secondary(pending)
                    .expect("view commits")
                    .id()
                    .clone(),
            );
        }
        assert_eq!(windows.open_count(), MAX_SESSION_WINDOWS);
        assert!(matches!(
            windows.prepare_secondary(ACTION_DOCUMENT),
            Err(UiWindowSessionError::OpenLimitReached)
        ));

        windows
            .close_secondary(&ids[1])
            .expect("a secondary can close");
        assert!(!windows.contains(&ids[1]));
        assert!(matches!(
            windows.replace_document(&ids[1], ACTION_DOCUMENT),
            Err(UiWindowSessionError::WindowUnavailable)
        ));
        let next = windows
            .prepare_secondary(ACTION_DOCUMENT)
            .expect("released capacity allows a different identity");
        assert_eq!(next.id().to_protocol_string(), "window-4");
        assert_eq!(
            windows.close_secondary(&UiWindowId::primary()),
            Err(UiWindowSessionError::PrimaryCannotClose)
        );
    }

    #[test]
    fn admits_only_one_pending_native_creation_and_allows_explicit_rollback() {
        let mut windows = UiWindowSessions::new();
        let pending = windows
            .prepare_secondary(ACTION_DOCUMENT)
            .expect("first native creation reserves its identity");
        assert!(matches!(
            windows.prepare_secondary(ACTION_DOCUMENT),
            Err(UiWindowSessionError::OpenBusy)
        ));
        windows
            .abort_secondary(pending)
            .expect("failed native creation releases its reservation");
        assert_eq!(
            windows
                .prepare_secondary(ACTION_DOCUMENT)
                .expect("the same unissued identity may be retried")
                .id()
                .to_protocol_string(),
            "window-1"
        );
    }
}
