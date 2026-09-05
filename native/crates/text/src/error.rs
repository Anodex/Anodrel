//! Closed failure categories for bounded text runs.

use anodrel_font::{FontKerningError, FontMetricError};

/// A complete text-run request could not produce one bounded run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextRunError {
    /// The selected face has no complete metric source or a mapped glyph is invalid there.
    Metric(FontMetricError),
    /// The selected face cannot use one supplied glyph in its pair source.
    Kerning(FontKerningError),
    /// One source scalar has no nonzero glyph in the selected face character map.
    GlyphUnavailable,
    /// The source value contains more than the fixed glyph-count limit.
    TooManyGlyphs,
    /// A glyph advance or pair adjustment would exceed the fixed position limit.
    AdvanceLimitExceeded,
}
