//! Immutable UI document snapshots for host-thread delivery.

use anodrel_ui::UiDocument;

use crate::UiDocumentRevision;

/// One validated UI document paired with its exact revision.
#[derive(Clone, Debug)]
pub struct UiDocumentSnapshot {
    document: UiDocument,
    revision: UiDocumentRevision,
}

impl UiDocumentSnapshot {
    pub(crate) fn new(document: UiDocument, revision: UiDocumentRevision) -> Self {
        Self { document, revision }
    }

    /// Returns the immutable document selected by this snapshot.
    #[must_use]
    pub const fn document(&self) -> &UiDocument {
        &self.document
    }

    /// Returns the exact revision that selected this document.
    #[must_use]
    pub const fn revision(&self) -> UiDocumentRevision {
        self.revision
    }
}
