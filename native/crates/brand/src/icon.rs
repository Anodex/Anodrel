//! Line glyphs for status cards and action tiles.
//!
//! Icons are open polylines in a normalised unit square, stroked at draw time.
//! Stroking rather than filling keeps one weight across the set, so glyphs stay
//! a family at any size instead of drifting apart as filled shapes would.

use anodrel_canvas::{Canvas, Paint, Point, Rect, point};

/// A glyph in the Anodrel icon set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    /// A cube: the owned protocol core.
    Core,
    /// A shield with a check: a verified package.
    Package,
    /// Two interlocking rings: the private transport.
    Ipc,
    /// A window frame: the native shell.
    Shell,
    /// An upward arrow: launching an application.
    Launch,
    /// A document with rules: runtime logs.
    Logs,
    /// A magnifier: inspecting package contents.
    Inspect,
    /// A pulse trace: runtime diagnostics.
    Diagnostics,
}

impl Icon {
    /// Returns the glyph as normalised polylines.
    ///
    /// Each polyline is an open run of points unless its first and last points
    /// coincide, which is how closed outlines are expressed without a separate
    /// flag.
    #[must_use]
    pub fn unit_polylines(self) -> Vec<Vec<Point>> {
        match self {
            Self::Core => vec![
                closed(vec![
                    point(0.50, 0.06),
                    point(0.94, 0.30),
                    point(0.94, 0.72),
                    point(0.50, 0.96),
                    point(0.06, 0.72),
                    point(0.06, 0.30),
                ]),
                vec![point(0.50, 0.51), point(0.50, 0.96)],
                vec![point(0.50, 0.51), point(0.06, 0.30)],
                vec![point(0.50, 0.51), point(0.94, 0.30)],
            ],
            Self::Package => vec![
                closed(vec![
                    point(0.50, 0.05),
                    point(0.90, 0.21),
                    point(0.88, 0.55),
                    point(0.72, 0.79),
                    point(0.50, 0.95),
                    point(0.28, 0.79),
                    point(0.12, 0.55),
                    point(0.10, 0.21),
                ]),
                vec![point(0.32, 0.47), point(0.45, 0.62), point(0.69, 0.33)],
            ],
            Self::Ipc => vec![
                circle(point(0.35, 0.50), 0.235),
                circle(point(0.65, 0.50), 0.235),
            ],
            Self::Shell => vec![
                closed(vec![
                    point(0.07, 0.15),
                    point(0.93, 0.15),
                    point(0.93, 0.85),
                    point(0.07, 0.85),
                ]),
                vec![point(0.07, 0.35), point(0.93, 0.35)],
                vec![point(0.38, 0.35), point(0.38, 0.85)],
            ],
            Self::Launch => vec![
                vec![point(0.50, 0.92), point(0.50, 0.16)],
                vec![point(0.24, 0.42), point(0.50, 0.13), point(0.76, 0.42)],
            ],
            Self::Logs => vec![
                closed(vec![
                    point(0.19, 0.07),
                    point(0.64, 0.07),
                    point(0.81, 0.25),
                    point(0.81, 0.93),
                    point(0.19, 0.93),
                ]),
                vec![point(0.64, 0.07), point(0.64, 0.25), point(0.81, 0.25)],
                vec![point(0.32, 0.45), point(0.68, 0.45)],
                vec![point(0.32, 0.61), point(0.68, 0.61)],
                vec![point(0.32, 0.77), point(0.55, 0.77)],
            ],
            Self::Inspect => vec![
                circle(point(0.43, 0.43), 0.31),
                vec![point(0.66, 0.66), point(0.92, 0.92)],
            ],
            Self::Diagnostics => vec![vec![
                point(0.05, 0.52),
                point(0.28, 0.52),
                point(0.40, 0.19),
                point(0.55, 0.83),
                point(0.67, 0.52),
                point(0.95, 0.52),
            ]],
        }
    }

    /// Strokes the glyph inside `bounds`.
    ///
    /// The glyph is fitted to the square inscribed in `bounds`, so a
    /// non-square target pads rather than distorting the artwork.
    pub fn draw(self, canvas: &mut Canvas, bounds: Rect, width: f32, paint: &Paint) {
        let side = bounds.width().min(bounds.height());
        if side <= 0.0 {
            return;
        }
        let square = Rect::centered(bounds.center(), side, side);
        for polyline in self.unit_polylines() {
            let placed: Vec<Point> = polyline
                .into_iter()
                .map(|vertex| point(square.left + vertex.x * side, square.top + vertex.y * side))
                .collect();
            canvas.draw_polyline(&placed, width, paint);
        }
    }
}

fn closed(mut points: Vec<Point>) -> Vec<Point> {
    if let Some(first) = points.first().copied() {
        points.push(first);
    }
    points
}

fn circle(center: Point, radius: f32) -> Vec<Point> {
    let steps = 28;
    let mut points: Vec<Point> = (0..steps)
        .map(|step| {
            let angle = std::f32::consts::TAU * (step as f32) / (steps as f32);
            point(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect();
    closed(std::mem::take(&mut points))
}

#[cfg(test)]
mod tests {
    use super::Icon;
    use anodrel_canvas::{Canvas, Color, Paint, Rect};

    const EVERY_ICON: [Icon; 8] = [
        Icon::Core,
        Icon::Package,
        Icon::Ipc,
        Icon::Shell,
        Icon::Launch,
        Icon::Logs,
        Icon::Inspect,
        Icon::Diagnostics,
    ];

    #[test]
    fn every_glyph_stays_inside_the_unit_square() {
        for icon in EVERY_ICON {
            for polyline in icon.unit_polylines() {
                for vertex in polyline {
                    assert!(
                        (0.0..=1.0).contains(&vertex.x) && (0.0..=1.0).contains(&vertex.y),
                        "{icon:?} has a vertex outside the unit square: {vertex:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_glyph_has_drawable_geometry() {
        for icon in EVERY_ICON {
            let polylines = icon.unit_polylines();
            assert!(!polylines.is_empty(), "{icon:?} has no polylines");
            for polyline in polylines {
                assert!(polyline.len() >= 2, "{icon:?} has a degenerate polyline");
            }
        }
    }

    #[test]
    fn drawing_marks_the_canvas_for_every_glyph() {
        for icon in EVERY_ICON {
            let mut canvas = Canvas::new(48, 48);
            canvas.clear(Color::BLACK);
            icon.draw(
                &mut canvas,
                Rect::new(4.0, 4.0, 44.0, 44.0),
                2.0,
                &Paint::solid(Color::WHITE),
            );
            let lit = (0..48)
                .flat_map(|y| (0..48).map(move |x| (x, y)))
                .filter(|(x, y)| canvas.pixel(*x, *y).red > 20)
                .count();
            assert!(lit > 40, "{icon:?} drew only {lit} pixels");
        }
    }

    #[test]
    fn a_non_square_target_pads_instead_of_distorting() {
        let mut wide = Canvas::new(96, 48);
        wide.clear(Color::BLACK);
        Icon::Core.draw(
            &mut wide,
            Rect::new(0.0, 0.0, 96.0, 48.0),
            2.0,
            &Paint::solid(Color::WHITE),
        );
        // The glyph is confined to the inscribed square, so the far edges stay clear.
        for y in 0..48 {
            assert_eq!(wide.pixel(1, y), Color::BLACK);
            assert_eq!(wide.pixel(94, y), Color::BLACK);
        }
    }

    #[test]
    fn an_empty_target_is_ignored() {
        let mut canvas = Canvas::new(16, 16);
        canvas.clear(Color::BLACK);
        Icon::Core.draw(
            &mut canvas,
            Rect::new(8.0, 8.0, 8.0, 8.0),
            2.0,
            &Paint::solid(Color::WHITE),
        );
        assert_eq!(canvas.pixel(8, 8), Color::BLACK);
    }
}
