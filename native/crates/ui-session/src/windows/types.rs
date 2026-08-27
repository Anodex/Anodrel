//! Public view-scoped values and failures for logical UI-window sessions.

use std::fmt;

use crate::{
    UiDocumentMailbox, UiDocumentSnapshot, UiInputBatch, UiInputMailbox, UiSessionError, UiWindowId,
};

use super::UiWindowState;

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
    pub(super) id: UiWindowId,
    pub(super) document_mailbox: UiDocumentMailbox,
    pub(super) input_mailbox: UiInputMailbox,
}

/// One view's bounded native semantic-input batch.
///
/// This is a host-internal grouping value. It does not provide application
/// window enumeration: a protocol operation decides separately whether and how
/// it exposes an accepted event.
#[derive(Debug)]
pub struct UiWindowInputBatch {
    pub(super) id: UiWindowId,
    pub(super) batch: UiInputBatch,
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
    pub(super) id: UiWindowId,
    pub(super) snapshot: UiDocumentSnapshot,
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
/// A host calls [`super::UiWindowSessions::abort_secondary`] after native
/// creation fails, leaving no routable identity or mailbox behind. It commits
/// the value only after it has created and registered the corresponding native
/// view.
#[derive(Debug)]
pub struct PendingUiWindow {
    pub(super) state: UiWindowState,
    pub(super) snapshot: UiDocumentSnapshot,
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
