//! Bounded display-only metadata retained by selected machine policy.

use std::fmt;

/// Bounded signed text for a host-owned product surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductDisplayMetadata {
    display_name: String,
    publisher_name: String,
}

/// A product display value did not meet the strict private-policy grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductDisplayMetadataError {
    /// One display value was empty, oversized, or unsafe for host presentation.
    Invalid,
}

impl fmt::Display for ProductDisplayMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("product display metadata is invalid")
    }
}

impl std::error::Error for ProductDisplayMetadataError {}

impl ProductDisplayMetadata {
    /// Creates one safe display-only product metadata value.
    pub fn new(
        display_name: &str,
        publisher_name: &str,
    ) -> Result<Self, ProductDisplayMetadataError> {
        if !is_safe_display_text(display_name) || !is_safe_display_text(publisher_name) {
            return Err(ProductDisplayMetadataError::Invalid);
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

#[cfg(test)]
mod tests {
    use super::{ProductDisplayMetadata, ProductDisplayMetadataError};

    #[test]
    fn display_metadata_rejects_directional_format_and_surrounding_whitespace() {
        assert!(ProductDisplayMetadata::new("Anodrel", "Anodrel").is_ok());
        for value in [" Anodrel", "Anodrel ", "Anodrel\u{202E}Test"] {
            assert_eq!(
                ProductDisplayMetadata::new(value, "Anodrel"),
                Err(ProductDisplayMetadataError::Invalid)
            );
        }
    }
}
