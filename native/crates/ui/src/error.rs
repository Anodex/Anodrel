//! Errors raised while building a UI document.

use std::fmt;

/// A validation failure in the bounded UI model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiError {
    /// An element ID is empty, too long, or has an unsupported character.
    InvalidElementId,
    /// Two nodes in one document have the same element ID.
    DuplicateElementId,
    /// Text or an action label is empty, too large, or not a single line.
    InvalidText,
    /// A font size is outside the supported logical-pixel range.
    InvalidFontSize,
    /// A padding or gap value exceeds the supported logical-pixel range.
    InvalidSpacing,
    /// A document contains more nodes than its bounded model permits.
    NodeLimitExceeded,
    /// A document nests nodes more deeply than its bounded model permits.
    DepthLimitExceeded,
    /// Combined text and label bytes exceed the document limit.
    TextLimitExceeded,
    /// A field's maximum length is outside the supported character range.
    InvalidFieldLength,
    /// A document contains more than one semantic status node.
    StatusLimitExceeded,
}

impl fmt::Display for UiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidElementId => "element ID is invalid",
            Self::DuplicateElementId => "document contains a duplicate element ID",
            Self::InvalidText => "text must be a non-empty single line within the size limit",
            Self::InvalidFontSize => "font size is outside the supported range",
            Self::InvalidSpacing => "padding or gap is outside the supported range",
            Self::NodeLimitExceeded => "document exceeds the node limit",
            Self::DepthLimitExceeded => "document exceeds the nesting-depth limit",
            Self::TextLimitExceeded => "document exceeds the combined text limit",
            Self::InvalidFieldLength => "field maximum length is outside the supported range",
            Self::StatusLimitExceeded => "document contains more than one status node",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UiError {}
