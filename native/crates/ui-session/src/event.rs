//! Application-facing semantic event data.

use anodrel_ui::ElementId;

use crate::UiDocumentRevision;

/// One validated semantic action from a current UI document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiApplicationEvent {
    revision: UiDocumentRevision,
    action: ElementId,
}

impl UiApplicationEvent {
    pub(crate) const fn new(revision: UiDocumentRevision, action: ElementId) -> Self {
        Self { revision, action }
    }

    /// Returns the exact document revision that produced this action.
    #[must_use]
    pub const fn revision(&self) -> UiDocumentRevision {
        self.revision
    }

    /// Returns the enabled action's stable semantic element ID.
    #[must_use]
    pub fn action(&self) -> &ElementId {
        &self.action
    }
}
