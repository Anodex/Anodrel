//! Stable identities for UI elements.

use crate::UiError;

/// A validated, document-unique semantic UI element identity.
///
/// IDs contain one through 64 ASCII bytes. They may use letters, digits,
/// periods, underscores, and hyphens, and they begin and end with a letter or
/// digit. [`UiDocument`](crate::UiDocument) additionally enforces uniqueness.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementId(String);

impl ElementId {
    /// Builds a validated element ID.
    ///
    /// The returned value is valid independently. It becomes document-unique
    /// only when it is used in a [`UiDocument`](crate::UiDocument).
    pub fn new(value: impl Into<String>) -> Result<Self, UiError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let is_valid = (1..=64).contains(&bytes.len())
            && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
            && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
            && bytes
                .iter()
                .copied()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if is_valid {
            Ok(Self(value))
        } else {
            Err(UiError::InvalidElementId)
        }
    }

    /// Returns the ID as validated text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ElementId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
