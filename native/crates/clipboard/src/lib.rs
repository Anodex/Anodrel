//! Bounded, portable clipboard text values.
//!
//! This crate owns no native clipboard handle or operating-system call. Native
//! adapters use these values to keep their public boundary UTF-8 and bounded.
//! See `docs/CLIPBOARD.md` and Decision 0040.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

/// Maximum UTF-8 bytes in one clipboard text value.
pub const MAX_CLIPBOARD_TEXT_BYTES: usize = 64 * 1024;

/// One validated clipboard text value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardText(String);

impl ClipboardText {
    /// Builds a text value after enforcing the clipboard byte limit.
    pub fn new(value: impl Into<String>) -> Result<Self, ClipboardInputError> {
        let value = value.into();
        if value.len() > MAX_CLIPBOARD_TEXT_BYTES {
            return Err(ClipboardInputError::TooLarge);
        }
        Ok(Self(value))
    }

    /// Returns the validated UTF-8 text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A stable result of reading the supported clipboard text form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClipboardRead {
    /// Unicode text was present, including an empty string.
    Text(ClipboardText),
    /// The clipboard has no supported Unicode text form.
    NoText,
}

/// A stable validation failure before a native clipboard call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardInputError {
    /// The UTF-8 input or returned value exceeds the fixed bound.
    TooLarge,
}

impl fmt::Display for ClipboardInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("clipboard text exceeds the fixed size limit")
    }
}

impl std::error::Error for ClipboardInputError {}

#[cfg(test)]
mod tests {
    use super::{ClipboardInputError, ClipboardRead, ClipboardText, MAX_CLIPBOARD_TEXT_BYTES};

    #[test]
    fn accepts_empty_and_utf8_text_inside_the_bound() {
        assert_eq!(ClipboardText::new(""), Ok(ClipboardText(String::new())));
        assert_eq!(ClipboardText::new("Año").unwrap().as_str(), "Año");
        assert_eq!(ClipboardRead::NoText, ClipboardRead::NoText);
    }

    #[test]
    fn rejects_only_values_larger_than_the_utf8_byte_limit() {
        assert!(ClipboardText::new("x".repeat(MAX_CLIPBOARD_TEXT_BYTES)).is_ok());
        assert_eq!(
            ClipboardText::new("x".repeat(MAX_CLIPBOARD_TEXT_BYTES + 1)),
            Err(ClipboardInputError::TooLarge)
        );
    }
}
