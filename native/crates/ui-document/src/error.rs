//! Stable failure categories for UI document interchange.

use std::fmt;

use anodrel_ui::UiError;

/// A safe category for a rejected UI document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiDocumentError {
    /// The encoded UTF-8 document exceeds the fixed size limit.
    EncodedLimitExceeded,
    /// The input is not one well-formed strict JSON value.
    InvalidJson,
    /// The root JSON value or an expected structural field is not an object.
    ExpectedObject,
    /// An expected structural field is not an array.
    ExpectedArray,
    /// A field is missing from an exact object schema.
    MissingField,
    /// An object contains a field not recognized by this format version.
    UnknownField,
    /// A field has an unsupported primitive type or value.
    InvalidField,
    /// The document declares an unsupported format identifier.
    UnsupportedFormat,
    /// The document describes an unsupported UI node kind.
    UnsupportedNodeKind,
    /// A structurally valid value violates the bounded UI model.
    InvalidModel(UiError),
}

impl From<UiError> for UiDocumentError {
    fn from(error: UiError) -> Self {
        Self::InvalidModel(error)
    }
}

impl fmt::Display for UiDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EncodedLimitExceeded => "UI document exceeds the encoded size limit",
            Self::InvalidJson => "UI document is not valid strict JSON",
            Self::ExpectedObject => "UI document expects an object",
            Self::ExpectedArray => "UI document expects an array",
            Self::MissingField => "UI document is missing a required field",
            Self::UnknownField => "UI document has an unsupported field",
            Self::InvalidField => "UI document has an invalid field value",
            Self::UnsupportedFormat => "UI document format is unsupported",
            Self::UnsupportedNodeKind => "UI document node kind is unsupported",
            Self::InvalidModel(error) => return error.fmt(formatter),
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UiDocumentError {}
