//! In-memory state for one caller-selected UI document session.

use anodrel_ui::{ElementId, UiDocument, UiEvent, UiNode};
use anodrel_ui_document::{decode, decode_v2};

use crate::{UiApplicationEvent, UiDocumentRevision, UiDocumentSnapshot, UiSessionError};

/// One revision-bound current UI document.
///
/// This type neither authenticates its caller nor performs I/O. A host or
/// transport creates one value only after it has made those lifecycle choices.
#[derive(Clone, Debug, Default)]
pub struct UiDocumentSession {
    revision: UiDocumentRevision,
    document: Option<UiDocument>,
}

impl UiDocumentSession {
    /// Builds an empty session at revision zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            revision: UiDocumentRevision::INITIAL,
            document: None,
        }
    }

    /// Returns the current immutable document and its exact revision.
    #[must_use]
    pub fn document(&self) -> Option<(&UiDocument, UiDocumentRevision)> {
        self.document
            .as_ref()
            .map(|document| (document, self.revision))
    }

    /// Clones the current document into a revision-bound delivery snapshot.
    #[must_use]
    pub fn snapshot(&self) -> Option<UiDocumentSnapshot> {
        self.document
            .as_ref()
            .map(|document| UiDocumentSnapshot::new(document.clone(), self.revision))
    }

    /// Validates and atomically replaces the current document.
    ///
    /// A failed decode leaves the current document and revision unchanged.
    pub fn replace_document(
        &mut self,
        encoded_document: &str,
    ) -> Result<UiDocumentRevision, UiSessionError> {
        self.replace_decoded(decode(encoded_document))
    }

    /// Validates and atomically replaces the current document from exact v2 data.
    ///
    /// This explicit opt-in accepts the version 2 scroll-container format only.
    /// It has the same revision and failure behavior as [`Self::replace_document`]
    /// and does not change the authenticated protocol operation's v1 contract.
    pub fn replace_document_v2(
        &mut self,
        encoded_document: &str,
    ) -> Result<UiDocumentRevision, UiSessionError> {
        self.replace_decoded(decode_v2(encoded_document))
    }

    fn replace_decoded(
        &mut self,
        decoded_document: Result<UiDocument, anodrel_ui_document::UiDocumentError>,
    ) -> Result<UiDocumentRevision, UiSessionError> {
        let document = decoded_document.map_err(UiSessionError::InvalidDocument)?;
        let revision = self
            .revision
            .next()
            .ok_or(UiSessionError::RevisionExhausted)?;
        self.document = Some(document);
        self.revision = revision;
        Ok(revision)
    }

    /// Removes the current document and invalidates its revision-bound events.
    ///
    /// Returns `Ok(None)` when the session was already empty.
    pub fn clear_document(&mut self) -> Result<Option<UiDocumentRevision>, UiSessionError> {
        if self.document.is_none() {
            return Ok(None);
        }
        let revision = self
            .revision
            .next()
            .ok_or(UiSessionError::RevisionExhausted)?;
        self.document = None;
        self.revision = revision;
        Ok(Some(revision))
    }

    /// Validates one semantic action against the current revision and document.
    ///
    /// The caller must derive `event` from the document's current visible
    /// layout. This type checks revision and enabled-action identity only; it
    /// cannot observe a host's geometry, focus, or native input lifecycle.
    pub fn accept_event(
        &self,
        revision: UiDocumentRevision,
        event: UiEvent,
    ) -> Result<UiApplicationEvent, UiSessionError> {
        if revision != self.revision {
            return Err(UiSessionError::StaleRevision);
        }
        let document = self
            .document
            .as_ref()
            .ok_or(UiSessionError::NoCurrentDocument)?;
        let UiEvent::ActionInvoked(action) = event;
        if !contains_enabled_action(document.root(), &action) {
            return Err(UiSessionError::ActionUnavailable);
        }
        Ok(UiApplicationEvent::new(revision, action))
    }
}

fn contains_enabled_action(node: &UiNode, expected: &ElementId) -> bool {
    match node {
        UiNode::Action(action) => action.enabled() && action.id() == expected,
        UiNode::Stack(stack) => stack
            .children()
            .iter()
            .any(|child| contains_enabled_action(child, expected)),
        UiNode::Scroll(scroll) => contains_enabled_action(scroll.child(), expected),
        // A field is not an action. Typing into one produces no event at all,
        // so a delivered event can never name a field — which also means an
        // application cannot use the event path to learn that a field was
        // touched. See Decision 0067.
        UiNode::Text(_) | UiNode::Field(_) => false,
    }
}
