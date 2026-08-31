//! Bounded conversion from owned glyph paths to canvas polygons.
//!
//! This crate is the explicit device-space boundary between `anodrel-font` and
//! `anodrel-canvas`. It does not load fonts, shape text, allocate coverage, or
//! call an operating-system API. See `docs/GLYPH_RENDERING.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod coverage;
mod error;
mod flatten;
mod placement;

pub use coverage::coverage_mask;
pub use error::GlyphRenderError;
pub use flatten::canvas_path;
pub use placement::GlyphPlacement;

#[cfg(test)]
mod tests;
