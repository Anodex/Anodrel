#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only access to the ordinary Windows Unicode-text clipboard.
//!
//! This adapter accepts and returns only the bounded portable types from
//! `anodrel-clipboard`. Native handles, format identifiers, and Windows error
//! codes do not cross this boundary. See `docs/CLIPBOARD.md`.

mod raw;

use anodrel_clipboard::{ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText};

/// Direct Windows clipboard service associated with one transient host window.
#[derive(Debug)]
pub struct WindowsClipboard {
    owner_window: isize,
}

impl WindowsClipboard {
    /// Creates a service using the current host window as clipboard owner.
    ///
    /// `owner_window` may be zero for a host that has no native window.
    #[must_use]
    pub const fn new(owner_window: isize) -> Self {
        Self { owner_window }
    }
}

impl ClipboardService for WindowsClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        read_text(self.owner_window)
    }

    fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        write_text(self.owner_window, text)
    }
}

/// Reads Unicode text from the Windows clipboard owned by `owner_window`.
///
/// `owner_window` is the current native host window handle. The value is used
/// only while opening the clipboard and is never retained by this adapter.
pub fn read_text(owner_window: isize) -> Result<ClipboardRead, ClipboardServiceError> {
    match raw::read_text(owner_window).map_err(ClipboardServiceError::from)? {
        Some(value) => ClipboardText::new(value)
            .map(ClipboardRead::Text)
            .map_err(|_| ClipboardServiceError::StoredTextTooLarge),
        None => Ok(ClipboardRead::NoText),
    }
}

/// Replaces the Windows Unicode-text clipboard contents with `text`.
///
/// The portable value is already bounded and valid UTF-8 before this adapter
/// opens the operating-system clipboard.
pub fn write_text(owner_window: isize, text: &ClipboardText) -> Result<(), ClipboardServiceError> {
    raw::write_text(owner_window, text.as_str()).map_err(ClipboardServiceError::from)
}

impl From<raw::ClipboardRawError> for ClipboardServiceError {
    fn from(error: raw::ClipboardRawError) -> Self {
        match error {
            raw::ClipboardRawError::Unavailable => ClipboardServiceError::Unavailable,
            raw::ClipboardRawError::InvalidText => ClipboardServiceError::StoredTextInvalid,
            raw::ClipboardRawError::TooLarge => ClipboardServiceError::StoredTextTooLarge,
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_clipboard::{ClipboardRead, ClipboardText};

    use super::WindowsClipboard;

    #[test]
    fn safe_errors_do_not_include_native_details() {
        assert_eq!(
            anodrel_clipboard::ClipboardServiceError::Unavailable.to_string(),
            "clipboard is unavailable"
        );
        assert_eq!(
            anodrel_clipboard::ClipboardServiceError::StoredTextInvalid.to_string(),
            "clipboard text is invalid"
        );
    }

    #[test]
    fn no_text_remains_distinct_from_an_empty_text_value() {
        let empty = ClipboardRead::Text(ClipboardText::new("").expect("empty text is valid"));
        assert_ne!(empty, ClipboardRead::NoText);
    }

    #[test]
    fn service_retains_only_its_transient_owner_value() {
        assert_eq!(
            format!("{:?}", WindowsClipboard::new(0)),
            "WindowsClipboard { owner_window: 0 }"
        );
    }
}
