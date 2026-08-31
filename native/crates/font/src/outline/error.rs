//! Closed errors for one requested glyph outline.

use std::fmt;

/// A requested TrueType glyph outline cannot be returned under the owned contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphOutlineError {
    /// The parsed face has no complete TrueType outline-table set.
    OutlineUnavailable,
    /// The requested glyph identifier is outside the face's declared glyph range.
    InvalidGlyphId,
    /// The glyph bytes do not meet the bounded simple-outline format.
    MalformedOutline,
    /// The simple glyph exceeds the parser's fixed contour or point limit.
    ComplexityLimitExceeded,
    /// The glyph is a composite, which this simple-outline slice deliberately omits.
    CompositeGlyphUnsupported,
}

impl fmt::Display for GlyphOutlineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::OutlineUnavailable => "font face has no supported outline source",
            Self::InvalidGlyphId => "font glyph identifier is outside the face",
            Self::MalformedOutline => "font glyph outline is malformed",
            Self::ComplexityLimitExceeded => "font glyph outline exceeds the fixed limit",
            Self::CompositeGlyphUnsupported => "composite font glyph outlines are unsupported",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for GlyphOutlineError {}
