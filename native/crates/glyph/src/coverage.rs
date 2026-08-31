//! Bounded raster coverage for one already-flattened glyph path.

use anodrel_canvas::Mask;
use anodrel_font::GlyphPath;

use crate::{GlyphPlacement, GlyphRenderError, canvas_path};

const MAX_GLYPH_COVERAGE_PIXELS: usize = 262_144;

/// Rasterizes one glyph into a bounded anti-aliased coverage mask.
///
/// The returned mask carries its canvas-space position and can be composited by
/// `anodrel-canvas`. A glyph whose flattened bounds exceed the fixed limit
/// returns a closed error before the coverage buffer is allocated.
pub fn coverage_mask(
    path: &GlyphPath,
    placement: GlyphPlacement,
) -> Result<Mask, GlyphRenderError> {
    let path = canvas_path(path, placement)?;
    Mask::for_path_bounded(&path, 0.0, MAX_GLYPH_COVERAGE_PIXELS)
        .ok_or(GlyphRenderError::TooComplex)
}
