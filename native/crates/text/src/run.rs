//! Owned source-order glyph values and their horizontal pen positions.

use anodrel_font::{FontFace, FontMetrics, GlyphId};

use crate::TextRunError;

/// The greatest number of Unicode scalars that one run accepts.
pub const MAX_RUN_GLYPHS: usize = 1_024;
/// The greatest horizontal advance that one run may have in font design units.
pub const MAX_RUN_ADVANCE_DESIGN_UNITS: i32 = 1_048_576;

/// One glyph's source-order position in an unscaled single-line run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunGlyph {
    glyph: GlyphId,
    pen_x: i32,
    advance_width: u16,
}

impl RunGlyph {
    /// Returns this glyph's validated face-local identifier.
    pub const fn glyph(self) -> GlyphId {
        self.glyph
    }

    /// Returns this glyph's horizontal pen position in font design units.
    pub const fn pen_x(self) -> i32 {
        self.pen_x
    }

    /// Returns this glyph's horizontal advance in font design units.
    pub const fn advance_width(self) -> u16 {
        self.advance_width
    }
}

/// One complete bounded, unshaped, single-line glyph run.
#[derive(Debug, Eq, PartialEq)]
pub struct TextRun {
    metrics: FontMetrics,
    glyphs: Vec<RunGlyph>,
    advance_width: i32,
}

impl TextRun {
    /// Builds one complete source-order run from a validated face and UTF-8 text.
    ///
    /// This maps exactly one scalar to exactly one nonzero glyph and uses only
    /// that glyph's horizontal advance and conventional pair kerning. It
    /// deliberately does not perform fallback, ligatures, shaping, or line
    /// breaking.
    pub fn build(face: &FontFace<'_>, text: &str) -> Result<Self, TextRunError> {
        let metrics = face.font_metrics().map_err(TextRunError::Metric)?;
        let uses_basic_latin_positioning = text.is_ascii();
        let mut glyphs = Vec::with_capacity(text.len().min(MAX_RUN_GLYPHS));
        let mut advance_width = 0_i32;
        let mut previous_glyph = None;

        for character in text.chars() {
            if glyphs.len() == MAX_RUN_GLYPHS {
                return Err(TextRunError::TooManyGlyphs);
            }
            let glyph = face
                .glyph_id(character)
                .ok_or(TextRunError::GlyphUnavailable)?;
            let metric = face
                .horizontal_metric(glyph)
                .map_err(TextRunError::Metric)?;
            if let Some(previous_glyph) = previous_glyph {
                let adjustment = if uses_basic_latin_positioning {
                    face.basic_latin_horizontal_kerning(previous_glyph, glyph)
                } else {
                    face.horizontal_kerning(previous_glyph, glyph)
                }
                .map_err(TextRunError::Kerning)?;
                advance_width = bounded_position(advance_width, adjustment)?;
            }
            glyphs.push(RunGlyph {
                glyph,
                pen_x: advance_width,
                advance_width: metric.advance_width(),
            });
            advance_width = bounded_position(advance_width, i32::from(metric.advance_width()))?;
            previous_glyph = Some(glyph);
        }

        Ok(Self {
            metrics,
            glyphs,
            advance_width,
        })
    }

    /// Returns the face-wide metrics that governed this run.
    pub const fn metrics(&self) -> FontMetrics {
        self.metrics
    }

    /// Returns the complete source-order glyph sequence.
    pub fn glyphs(&self) -> &[RunGlyph] {
        &self.glyphs
    }

    /// Returns the horizontal position immediately after the final glyph.
    pub const fn advance_width(&self) -> i32 {
        self.advance_width
    }
}

/// Adds one bounded signed design-unit placement value without partial output.
fn bounded_position(position: i32, adjustment: i32) -> Result<i32, TextRunError> {
    let next = i64::from(position) + i64::from(adjustment);
    let limit = i64::from(MAX_RUN_ADVANCE_DESIGN_UNITS);
    if !(-limit..=limit).contains(&next) {
        return Err(TextRunError::AdvanceLimitExceeded);
    }
    Ok(next as i32)
}
