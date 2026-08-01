//! Directional edge shading, the effect that gives the Anodrel mark its facets.

use crate::color::Color;
use crate::geometry::Point;
use crate::paint::Paint;
use crate::path::Path;
use crate::surface::Canvas;

/// How a shape's edges respond to a single directional light.
///
/// The model is deliberately flat rather than a true renderer: every edge is
/// treated as a chamfer of constant depth, and its shading comes from the angle
/// between the edge's outward normal and the light. That is enough to make a
/// two-dimensional polygon read as a solid extruded piece, and it costs one
/// dot product per edge.
#[derive(Clone, Copy, Debug)]
pub struct Bevel {
    /// Width of the chamfer band, in pixels.
    pub depth: f32,
    /// Unit vector pointing from the surface toward the light.
    pub light: Point,
    /// Peak opacity of the white overlay on light-facing edges, `0.0..=1.0`.
    pub highlight: f32,
    /// Peak opacity of the black overlay on edges facing away, `0.0..=1.0`.
    pub shadow: f32,
}

impl Bevel {
    /// A bevel lit from the upper left, matching the Anodrel brand mark.
    #[must_use]
    pub fn top_left(depth: f32) -> Self {
        Self {
            depth,
            light: Point::new(-0.5145, -0.8575),
            highlight: 0.55,
            shadow: 0.45,
        }
    }

    /// Returns a copy with new highlight and shadow strengths.
    #[must_use]
    pub fn with_strength(self, highlight: f32, shadow: f32) -> Self {
        Self {
            highlight,
            shadow,
            ..self
        }
    }

    /// Returns a copy with every strength scaled, for fading a mark in.
    #[must_use]
    pub fn scaled(self, factor: f32) -> Self {
        Self {
            highlight: self.highlight * factor,
            shadow: self.shadow * factor,
            ..self
        }
    }

    /// Returns the overlay colour for an edge with the given outward normal.
    ///
    /// Returns `None` when the edge is edge-on to the light and needs no
    /// overlay at all.
    #[must_use]
    pub fn overlay_for(&self, outward_normal: Point) -> Option<Color> {
        let intensity = outward_normal.dot(self.light);
        if intensity > 0.0 {
            let alpha = (intensity * self.highlight * 255.0).round();
            (alpha >= 1.0).then(|| Color::WHITE.with_alpha(alpha.min(255.0) as u8))
        } else if intensity < 0.0 {
            let alpha = (-intensity * self.shadow * 255.0).round();
            (alpha >= 1.0).then(|| Color::BLACK.with_alpha(alpha.min(255.0) as u8))
        } else {
            None
        }
    }
}

impl Canvas {
    /// Fills a shape and shades its edges as chamfers lit from one direction.
    ///
    /// The face is painted first, then each chamfer band is composited over it.
    /// Bands are mitred, so they tile the border exactly once — translucent
    /// overlays never double up at a corner.
    pub fn fill_beveled(&mut self, path: &Path, face: &Paint, bevel: &Bevel) {
        self.fill_path(path, face);
        if bevel.depth <= 0.0 {
            return;
        }
        for band in path.bevel_bands(bevel.depth) {
            if let Some(overlay) = bevel.overlay_for(band.outward_normal) {
                self.fill_path(&band.quad, &Paint::solid(overlay));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bevel;
    use crate::color::Color;
    use crate::geometry::{Point, Rect, point};
    use crate::paint::Paint;
    use crate::path::Path;
    use crate::surface::Canvas;

    #[test]
    fn an_edge_facing_the_light_takes_a_white_overlay() {
        let bevel = Bevel::top_left(4.0);
        let overlay = bevel
            .overlay_for(Point::new(0.0, -1.0))
            .expect("an upward edge faces a light from above");
        assert_eq!(overlay.red, 255);
        assert!(overlay.alpha > 0);
    }

    #[test]
    fn an_edge_facing_away_takes_a_black_overlay() {
        let bevel = Bevel::top_left(4.0);
        let overlay = bevel
            .overlay_for(Point::new(0.0, 1.0))
            .expect("a downward edge faces away from a light above");
        assert_eq!(overlay.red, 0);
        assert!(overlay.alpha > 0);
    }

    #[test]
    fn a_bevelled_square_is_lighter_on_top_than_on_the_bottom() {
        let mut canvas = Canvas::new(64, 64);
        canvas.clear(Color::BLACK);
        let face = Paint::solid(Color::rgb(120, 120, 120));
        canvas.fill_beveled(
            &Path::rect(Rect::new(8.0, 8.0, 56.0, 56.0)),
            &face,
            &Bevel::top_left(6.0),
        );
        let top = canvas.pixel(32, 10).red;
        let middle = canvas.pixel(32, 32).red;
        let bottom = canvas.pixel(32, 53).red;
        assert!(
            top > middle,
            "top edge {top} should be lit above face {middle}"
        );
        assert!(
            bottom < middle,
            "bottom edge {bottom} should be below face {middle}"
        );
    }

    #[test]
    fn a_zero_depth_bevel_leaves_the_face_flat() {
        let mut canvas = Canvas::new(32, 32);
        canvas.clear(Color::BLACK);
        let face = Paint::solid(Color::rgb(100, 100, 100));
        canvas.fill_beveled(
            &Path::rect(Rect::new(4.0, 4.0, 28.0, 28.0)),
            &face,
            &Bevel::top_left(0.0),
        );
        assert_eq!(canvas.pixel(16, 5), canvas.pixel(16, 26));
    }

    #[test]
    fn scaling_a_bevel_to_zero_removes_every_overlay() {
        let faded = Bevel::top_left(4.0).scaled(0.0);
        assert!(faded.overlay_for(point(0.0, -1.0)).is_none());
        assert!(faded.overlay_for(point(0.0, 1.0)).is_none());
    }
}
