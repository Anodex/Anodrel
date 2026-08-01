//! Stable failure categories for UI document session state.

use std::fmt;

use anodrel_ui_document::UiDocumentError;

/// A safe category for a rejected UI document session operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiSessionError {
    /// The replacement document did not meet the strict interchange contract.
    InvalidDocument(UiDocumentError),
    /// The monotonic document revision cannot advance without wrapping.
    RevisionExhausted,
    /// No current document exists for an action submission.
    NoCurrentDocument,
    /// The action was produced by a previous document revision.
    StaleRevision,
    /// The current document does not contain an enabled action with this ID.
    ActionUnavailable,
}

impl fmt::Display for UiSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidDocument(error) => return error.fmt(formatter),
            Self::RevisionExhausted => "UI document revision space is exhausted",
            Self::NoCurrentDocument => "UI session has no current document",
            Self::StaleRevision => "UI event belongs to a stale document revision",
            Self::ActionUnavailable => "UI event does not name a current enabled action",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UiSessionError {}
