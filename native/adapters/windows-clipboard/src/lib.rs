#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only access to the ordinary Windows Unicode-text clipboard.
//!
//! This adapter accepts and returns only the bounded portable types from
//! `anodrel-clipboard`. Native handles, format identifiers, and Windows error
//! codes do not cross this boundary. See `docs/CLIPBOARD.md`.

mod raw;

use std::fmt;

use anodrel_clipboard::{ClipboardRead, ClipboardText};

/// Reads Unicode text from the Windows clipboard owned by `owner_window`.
///
/// `owner_window` is the current native host window handle. The value is used
/// only while opening the clipboard and is never retained by this adapter.
pub fn read_text(owner_window: isize) -> Result<ClipboardRead, ClipboardError> {
    match raw::read_text(owner_window).map_err(ClipboardError::from)? {
        Some(value) => ClipboardText::new(value)
            .map(ClipboardRead::Text)
            .map_err(|_| ClipboardError::StoredTextTooLarge),
        None => Ok(ClipboardRead::NoText),
    }
}

/// Replaces the Windows Unicode-text clipboard contents with `text`.
///
/// The portable value is already bounded and valid UTF-8 before this adapter
/// opens the operating-system clipboard.
pub fn write_text(owner_window: isize, text: &ClipboardText) -> Result<(), ClipboardError> {
    raw::write_text(owner_window, text.as_str()).map_err(ClipboardError::from)
}

/// A safe category for a Windows clipboard failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardError {
    /// The clipboard is currently unavailable, including contention.
    Unavailable,
    /// Windows returned malformed Unicode text.
    StoredTextInvalid,
    /// Windows returned Unicode text that exceeds Anodrel's portable limit.
    StoredTextTooLarge,
}

impl From<raw::ClipboardRawError> for ClipboardError {
    fn from(error: raw::ClipboardRawError) -> Self {
        match error {
            raw::ClipboardRawError::Unavailable => Self::Unavailable,
            raw::ClipboardRawError::InvalidText => Self::StoredTextInvalid,
            raw::ClipboardRawError::TooLarge => Self::StoredTextTooLarge,
        }
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unavailable => "Windows clipboard is unavailable",
            Self::StoredTextInvalid => "Windows clipboard text is invalid",
            Self::StoredTextTooLarge => "Windows clipboard text is too large",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ClipboardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use anodrel_clipboard::{ClipboardRead, ClipboardText};

    use super::ClipboardError;

    #[test]
    fn safe_errors_do_not_include_native_details() {
        assert_eq!(
            ClipboardError::Unavailable.to_string(),
            "Windows clipboard is unavailable"
        );
        assert_eq!(
            ClipboardError::StoredTextInvalid.to_string(),
            "Windows clipboard text is invalid"
        );
    }

    #[test]
    fn no_text_remains_distinct_from_an_empty_text_value() {
        let empty = ClipboardRead::Text(ClipboardText::new("").expect("empty text is valid"));
        assert_ne!(empty, ClipboardRead::NoText);
    }
}
