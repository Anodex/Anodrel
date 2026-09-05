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
    /// A composite component attaches points, which this translation-only slice omits.
    CompositePointAttachmentUnsupported,
    /// A composite component applies a transform or scaled offset, which this slice omits.
    CompositeTransformUnsupported,
    /// A composite component moves geometry outside signed font design units.
    CompositeCoordinateOutOfRange,
    /// A composite component graph references one of its active ancestors.
    CompositeCycle,
}

impl fmt::Display for GlyphOutlineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::OutlineUnavailable => "font face has no supported outline source",
            Self::InvalidGlyphId => "font glyph identifier is outside the face",
            Self::MalformedOutline => "font glyph outline is malformed",
            Self::ComplexityLimitExceeded => "font glyph outline exceeds the fixed limit",
            Self::CompositePointAttachmentUnsupported => {
                "composite font glyph point attachment is unsupported"
            }
            Self::CompositeTransformUnsupported => {
                "composite font glyph transforms are unsupported"
            }
            Self::CompositeCoordinateOutOfRange => {
                "composite font glyph coordinates exceed supported design units"
            }
            Self::CompositeCycle => "composite font glyph graph contains a cycle",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for GlyphOutlineError {}
