//! Explicit mapping from TrueType design coordinates into canvas pixels.

use anodrel_canvas::Point;
use anodrel_font::GlyphPathPoint;

use crate::GlyphRenderError;

const MAX_BASELINE_ABS: f32 = 1_048_576.0;
const MAX_PIXELS_PER_DESIGN_UNIT: f32 = 64.0;

/// One validated baseline and scale for a glyph conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphPlacement {
    baseline: Point,
    pixels_per_design_unit: f32,
}

impl GlyphPlacement {
    /// Builds a placement from a canvas-space baseline and a positive design-unit scale.
    pub fn new(baseline: Point, pixels_per_design_unit: f32) -> Result<Self, GlyphRenderError> {
        if !baseline.x.is_finite()
            || !baseline.y.is_finite()
            || baseline.x.abs() > MAX_BASELINE_ABS
            || baseline.y.abs() > MAX_BASELINE_ABS
            || !pixels_per_design_unit.is_finite()
            || !(0.0..=MAX_PIXELS_PER_DESIGN_UNIT).contains(&pixels_per_design_unit)
            || pixels_per_design_unit == 0.0
        {
            return Err(GlyphRenderError::InvalidPlacement);
        }
        Ok(Self {
            baseline,
            pixels_per_design_unit,
        })
    }

    pub(crate) fn map(self, point: GlyphPathPoint) -> Point {
        self.map_doubled(point.x_twice(), point.y_twice())
    }

    pub(crate) fn map_doubled(self, x_twice: i32, y_twice: i32) -> Point {
        let scale = self.pixels_per_design_unit * 0.5;
        Point::new(
            self.baseline.x + (x_twice as f32) * scale,
            self.baseline.y - (y_twice as f32) * scale,
        )
    }
}
