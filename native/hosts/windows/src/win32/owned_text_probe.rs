//! One fixed, non-windowed proof that the owned text layers compose together.
//!
//! This is intentionally not the Windows text painter. It accepts no caller
//! value and changes no host surface; see Decision 0212 for its fixed boundary.

use anodrel_canvas::{Canvas, Color, Paint, point};
use anodrel_font::FontFace;
use anodrel_glyph::GlyphMaskCache;
use anodrel_text::TextRun;

use super::selected_font;

const PROBE_TEXT: &str = "ANODREL";
const PROBE_EM_PIXELS: f32 = 32.0;
const PROBE_WIDTH: u32 = 512;
const PROBE_HEIGHT: u32 = 144;
const PROBE_LEFT: f32 = 24.0;
const PROBE_BASELINES: [f32; 2] = [48.0, 112.0];

/// Private closed outcomes from the fixed owned-text composition probe.
#[derive(Debug)]
pub(super) enum OwnedTextProbeError {
    /// Windows did not provide the fixed selected face under its source bounds.
    Source,
    /// The selected face failed the owned parser's validation.
    InvalidFace,
    /// The fixed source value could not form one owned text run.
    Run,
    /// One owned glyph could not provide bounded cached coverage.
    Glyph,
}

/// Private result facts from one fixed owned-text composition.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct OwnedTextProbe {
    pub(super) canvas: Canvas,
    pub(super) glyph_count: usize,
    pub(super) retained_mask_count: usize,
    pub(super) retained_pixel_count: usize,
}

/// Composes the fixed probe through every current owned text layer exactly twice.
///
/// No host route invokes this during normal presentation. The second row has an
/// integer-only baseline change, which must reuse the first row's cached masks.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn compose() -> Result<OwnedTextProbe, OwnedTextProbeError> {
    let source = selected_font::selected_face_data().ok_or(OwnedTextProbeError::Source)?;
    let face = FontFace::parse(&source).map_err(|_| OwnedTextProbeError::InvalidFace)?;
    let run = TextRun::build(&face, PROBE_TEXT).map_err(|_| OwnedTextProbeError::Run)?;
    let pixels_per_design_unit = PROBE_EM_PIXELS / f32::from(run.metrics().units_per_em());
    let mut cache = GlyphMaskCache::new(&face);
    let mut canvas = Canvas::new(PROBE_WIDTH, PROBE_HEIGHT);
    canvas.clear(Color::BLACK);

    for baseline_y in PROBE_BASELINES {
        for glyph in run.glyphs() {
            let baseline = point(
                PROBE_LEFT + glyph.pen_x() as f32 * pixels_per_design_unit,
                baseline_y,
            );
            let mask = cache
                .mask_at(glyph.glyph(), baseline, pixels_per_design_unit)
                .map_err(|_| OwnedTextProbeError::Glyph)?;
            canvas.fill_mask_offset(
                mask.mask(),
                mask.offset_x(),
                mask.offset_y(),
                &Paint::solid(Color::WHITE),
            );
        }
    }

    Ok(OwnedTextProbe {
        canvas,
        glyph_count: run.glyphs().len(),
        retained_mask_count: cache.retained_mask_count(),
        retained_pixel_count: cache.retained_pixel_count(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Color, PROBE_TEXT, compose};

    #[test]
    fn selected_face_completes_the_owned_text_chain_and_reuses_coverage() {
        let probe = compose().expect("selected face completes the owned text probe");
        assert_eq!(probe.glyph_count, PROBE_TEXT.chars().count());
        assert_eq!(probe.retained_mask_count, probe.glyph_count);
        assert!(probe.retained_pixel_count > 0);

        let black = Color::BLACK.to_argb();
        let row_has_ink = |top, bottom| {
            (top..bottom).any(|y| {
                let width = probe.canvas.width() as usize;
                probe.canvas.pixels()[y * width..(y + 1) * width]
                    .iter()
                    .any(|pixel| *pixel != black)
            })
        };
        assert!(row_has_ink(0, 64));
        assert!(row_has_ink(64, probe.canvas.height() as usize));
    }
}
