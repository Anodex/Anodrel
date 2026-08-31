//! Closed outcomes from glyph preparation.

/// A safe failure from conversion of one glyph path into a canvas path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphRenderError {
    /// The supplied baseline or design-unit scale was outside the fixed bounds.
    InvalidPlacement,
    /// The fixed quality target could not be met within the glyph work limits.
    TooComplex,
}
