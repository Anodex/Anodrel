//! Shared fixed-input composition helpers for owned Windows text diagnostics.
//!
//! The helpers deliberately know nothing about a window, cache lifetime, or
//! visible painter selection. Callers own those policies while this module
//! keeps the portable run-to-mask sequence identical across fixed diagnostics.

use anodrel_canvas::{Canvas, Paint, Point, point};
use anodrel_glyph::{GlyphCacheError, GlyphMaskCache};
use anodrel_text::TextRun;

/// Converts one fixed em size into the owned run's explicit device scale.
pub(super) fn pixels_per_design_unit(run: &TextRun, em_pixels: f32) -> Option<f32> {
    let scale = em_pixels / f32::from(run.metrics().units_per_em());
    (scale.is_finite() && scale > 0.0).then_some(scale)
}

/// Draws one complete owned run through a caller-owned face-local cache.
pub(super) fn draw_row(
    canvas: &mut Canvas,
    cache: &mut GlyphMaskCache<'_>,
    run: &TextRun,
    pixels_per_design_unit: f32,
    baseline: Point,
    paint: &Paint,
) -> Result<(), GlyphCacheError> {
    for glyph in run.glyphs() {
        let glyph_baseline = point(
            baseline.x + glyph.pen_x() as f32 * pixels_per_design_unit,
            baseline.y,
        );
        let mask = cache.mask_at(glyph.glyph(), glyph_baseline, pixels_per_design_unit)?;
        canvas.fill_mask_offset(mask.mask(), mask.offset_x(), mask.offset_y(), paint);
    }
    Ok(())
}

/// Converts a bounded owned run advance to thousandths of a physical pixel.
pub(super) fn advance_milli_pixels(run: &TextRun, pixels_per_design_unit: f32) -> Option<i64> {
    let milli_pixels = f64::from(run.advance_width()) * f64::from(pixels_per_design_unit) * 1_000.0;
    (milli_pixels.is_finite() && (0.0..=i64::MAX as f64).contains(&milli_pixels))
        .then(|| milli_pixels.round() as i64)
}
