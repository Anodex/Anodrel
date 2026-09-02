//! Bounded signed display metadata for host-owned product surfaces.

use crate::ReleaseManifestError;

/// Bounded signed text for host-owned Windows product surfaces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductMetadata {
    display_name: String,
    publisher_name: String,
}

impl ProductMetadata {
    /// Creates one safe display-only product metadata value.
    pub fn new(display_name: &str, publisher_name: &str) -> Result<Self, ReleaseManifestError> {
        if !is_safe_display_text(display_name) || !is_safe_display_text(publisher_name) {
            return Err(ReleaseManifestError::ProductMetadataInvalid);
        }
        Ok(Self {
            display_name: display_name.to_owned(),
            publisher_name: publisher_name.to_owned(),
        })
    }

    /// Returns the signed product display name for a host-owned surface.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the signed publisher display name for a host-owned surface.
    #[must_use]
    pub fn publisher_name(&self) -> &str {
        &self.publisher_name
    }
}

fn is_safe_display_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value == value.trim()
        && value.chars().all(is_safe_display_character)
}

fn is_safe_display_character(character: char) -> bool {
    !character.is_control()
        && !matches!(
            character,
            '\u{061C}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
        )
}
