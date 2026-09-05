//! Closed failure categories for bounded text runs.

use anodrel_font::FontMetricError;

/// A complete text-run request could not produce one bounded run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextRunError {
    /// The selected face has no complete metric source or a mapped glyph is invalid there.
    Metric(FontMetricError),
    /// One source scalar has no nonzero glyph in the selected face character map.
    GlyphUnavailable,
    /// The source value contains more than the fixed glyph-count limit.
    TooManyGlyphs,
    /// Adding a glyph advance would exceed the fixed design-unit limit.
    AdvanceLimitExceeded,
}
