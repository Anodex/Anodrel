//! Closed failure categories for face parsing.

use std::fmt;

/// A face or selected character map did not meet the bounded font contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontError {
    /// The SFNT header or table directory is malformed or out of bounds.
    InvalidFace,
    /// The face uses an SFNT flavour this initial parser does not support.
    UnsupportedFace,
    /// The face has no `cmap` table.
    MissingCharacterMap,
    /// The selected Unicode character map is malformed or out of bounds.
    InvalidCharacterMap,
    /// The face has no Unicode format-4 or format-12 map this parser supports.
    UnsupportedCharacterMap,
}

impl fmt::Display for FontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidFace => "font face is malformed",
            Self::UnsupportedFace => "font face format is unsupported",
            Self::MissingCharacterMap => "font face has no character map",
            Self::InvalidCharacterMap => "font character map is malformed",
            Self::UnsupportedCharacterMap => "font character map is unsupported",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for FontError {}
