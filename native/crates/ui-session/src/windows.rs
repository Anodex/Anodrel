//! Bounded, logical multi-view state for one authenticated UI session.
//!
//! This module owns no native windows, transport endpoint, application identity,
//! or permission decision. It is the portable state beneath the future
//! session-owned multi-window host contract in `docs/MULTI_WINDOW.md`.

use std::{collections::BTreeMap, num::NonZeroU16};

use anodrel_ui::UiEvent;

use crate::{
    UiApplicationEvent, UiDocumentMailbox, UiDocumentRevision, UiDocumentSession, UiInputBatch,
    UiInputMailbox, UiWindowId,
};

#[cfg(test)]
use crate::UiSessionError;

mod types;

pub use types::{
    MAX_SESSION_WINDOWS, PendingUiWindow, UiWindowInputBatch, UiWindowResources,
    UiWindowSessionError, UiWindowSnapshot,
};

#[derive(Clone, Copy)]
enum InitialDocumentFormat {
    V1,
    V2,
    V3,
}

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
        self.prepare_secondary_with(encoded_document, InitialDocumentFormat::V1)
    }

    /// Validates an initial v2 document and reserves the next secondary view.
    ///
    /// The version is selected before a native creation request exists, so a
    /// v1 or v3 route cannot accidentally accept a scroll document.
    pub fn prepare_secondary_v2(
        &mut self,
        encoded_document: &str,
    ) -> Result<PendingUiWindow, UiWindowSessionError> {
        self.prepare_secondary_with(encoded_document, InitialDocumentFormat::V2)
    }

    /// Validates an initial v3 document and reserves the next secondary view.
    ///
    /// The version is selected before a native creation request exists, so a
    /// v1 route cannot accidentally accept a document with status semantics.
    pub fn prepare_secondary_v3(
        &mut self,
        encoded_document: &str,
    ) -> Result<PendingUiWindow, UiWindowSessionError> {
        self.prepare_secondary_with(encoded_document, InitialDocumentFormat::V3)
    }

    fn prepare_secondary_with(
        &mut self,
        encoded_document: &str,
        format: InitialDocumentFormat,
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
        let replacement = match format {
            InitialDocumentFormat::V1 => state.document_session.replace_document(encoded_document),
            InitialDocumentFormat::V2 => {
                state.document_session.replace_document_v2(encoded_document)
            }
            InitialDocumentFormat::V3 => {
                state.document_session.replace_document_v3(encoded_document)
            }
        };
        let revision = replacement.map_err(UiWindowSessionError::DocumentRejected)?;
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
    /// Its exact v2 protocol route remains separate from the v1 and v3
    /// multi-window request contracts.
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

    /// Replaces one currently open view's explicit v3 document and publishes it.
    ///
    /// This preserves the exact versioned document boundary for live-status
    /// semantics; v1 and v2 replacement remain unchanged.
    pub fn replace_document_v3(
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
            .replace_document_v3(encoded_document)
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
mod tests;
