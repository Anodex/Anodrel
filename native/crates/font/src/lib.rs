//! Bounded first-party parsing for one caller-owned TrueType font face.
//!
//! The crate maps one Unicode scalar value to a nonzero glyph identifier. It
//! performs no font discovery, file access, fallback, shaping, measurement,
//! outline loading, or rasterization. See `docs/FONTS.md` for the boundary.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bytes;
mod cmap;
mod error;
mod face;
mod gpos;
mod kerning;
mod metrics;
mod outline;

pub use error::FontError;
pub use face::{FontFace, GlyphId};
pub use kerning::FontKerningError;
pub use metrics::{FontMetricError, FontMetrics, HorizontalMetric};
pub use outline::{
    GlyphBounds, GlyphOutline, GlyphOutlineError, GlyphPath, GlyphPathPoint, GlyphPoint,
    GlyphSegment,
};

#[cfg(test)]
mod tests;
