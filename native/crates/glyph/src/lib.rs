//! Bounded conversion from owned glyph paths to canvas polygons.
//!
//! This crate is the explicit device-space boundary between `anodrel-font` and
//! `anodrel-canvas`. It does not load fonts, shape text, or call an
//! operating-system API. See `docs/GLYPH_RENDERING.md` and `docs/GLYPH_CACHE.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod cache;
mod coverage;
mod error;
mod flatten;
mod placement;

pub use cache::{
    CachedGlyphMask, GlyphCacheError, GlyphMaskCache, MAX_CACHED_GLYPH_MASKS,
    MAX_CACHED_GLYPH_PIXELS,
};
pub use coverage::coverage_mask;
pub use error::GlyphRenderError;
pub use flatten::canvas_path;
pub use placement::GlyphPlacement;

#[cfg(test)]
mod tests;
