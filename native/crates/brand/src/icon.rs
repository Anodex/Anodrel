//! Line glyphs for status cards and action tiles.
//!
//! Icons are open polylines in a normalised unit square, stroked at draw time.
//! Stroking rather than filling keeps one weight across the set, so glyphs stay
//! a family at any size instead of drifting apart as filled shapes would.

use anodrel_canvas::{Canvas, Paint, Point, Rect, point};

/// A glyph in the Anodrel icon set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    /// A cube: the platform protocol core.
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

    /// Returns the glyph's own extent inside the unit square.
    ///
    /// Glyphs are authored to fit, not to fill: a shield reaches nearly top to
    /// bottom while two side-by-side rings occupy a narrow band. The raw
    /// coordinates therefore say nothing about how large a glyph looks.
    #[must_use]
    pub fn unit_bounds(self) -> Rect {
        let mut bounds: Option<Rect> = None;
        for polyline in self.unit_polylines() {
            for vertex in polyline {
                bounds = Some(match bounds {
                    None => Rect::new(vertex.x, vertex.y, vertex.x, vertex.y),
                    Some(current) => Rect::new(
                        current.left.min(vertex.x),
                        current.top.min(vertex.y),
                        current.right.max(vertex.x),
                        current.bottom.max(vertex.y),
                    ),
                });
            }
        }
        bounds.unwrap_or_default()
    }

    /// Strokes the glyph inside `bounds`.
    ///
    /// The glyph is scaled so its **own** longest dimension spans the square
    /// inscribed in `bounds`, then centred there. Placing raw coordinates
    /// instead would render each glyph at whatever size it happened to be
    /// authored at, which is what makes an icon set look mismatched inside
    /// identical containers.
    ///
    /// Aspect ratio is preserved, so a wide glyph pads vertically rather than
    /// stretching, and a non-square target pads rather than distorting.
    pub fn draw(self, canvas: &mut Canvas, bounds: Rect, width: f32, paint: &Paint) {
        let side = bounds.width().min(bounds.height());
        if side <= 0.0 {
            return;
        }
        let glyph = self.unit_bounds();
        let extent = glyph.width().max(glyph.height());
        if extent <= 0.0 {
            return;
        }
        let scale = side / extent;
        let center = bounds.center();
        let origin = glyph.center();
        for polyline in self.unit_polylines() {
            let placed: Vec<Point> = polyline
                .into_iter()
                .map(|vertex| {
                    point(
                        center.x + (vertex.x - origin.x) * scale,
                        center.y + (vertex.y - origin.y) * scale,
                    )
                })
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

    /// Returns the bounding box of drawn pixels, as (left, top, right, bottom).
    fn drawn_extent(icon: Icon, box_side: f32) -> (i32, i32, i32, i32) {
        let canvas_side = (box_side * 3.0) as u32;
        let mut canvas = Canvas::new(canvas_side, canvas_side);
        canvas.clear(Color::BLACK);
        let center = canvas_side as f32 / 2.0;
        icon.draw(
            &mut canvas,
            Rect::centered(anodrel_canvas::point(center, center), box_side, box_side),
            1.5,
            &Paint::solid(Color::WHITE),
        );
        let lit: Vec<(i32, i32)> = (0..canvas_side as i32)
            .flat_map(|y| (0..canvas_side as i32).map(move |x| (x, y)))
            .filter(|(x, y)| canvas.pixel(*x, *y).red > 40)
            .collect();
        assert!(!lit.is_empty(), "{icon:?} drew nothing");
        (
            lit.iter().map(|(x, _)| *x).min().expect("drew"),
            lit.iter().map(|(_, y)| *y).min().expect("drew"),
            lit.iter().map(|(x, _)| *x).max().expect("drew"),
            lit.iter().map(|(_, y)| *y).max().expect("drew"),
        )
    }

    #[test]
    fn every_glyph_renders_at_the_same_optical_size() {
        // Glyphs sit inside identical circles on the Startup Lab's cards. If
        // one is authored shorter than another, it renders smaller and the set
        // looks mismatched — which is exactly what happened before glyphs were
        // normalised to their own extent at draw time.
        let box_side = 60.0;
        for icon in EVERY_ICON {
            let (left, top, right, bottom) = drawn_extent(icon, box_side);
            let extent = (right - left).max(bottom - top) as f32;
            assert!(
                (extent - box_side).abs() <= 4.0,
                "{icon:?} spans {extent} where every glyph should span about {box_side}"
            );
        }
    }

    #[test]
    fn every_glyph_is_centred_in_its_target() {
        let box_side = 60.0;
        for icon in EVERY_ICON {
            let (left, top, right, bottom) = drawn_extent(icon, box_side);
            let target = box_side * 3.0 / 2.0;
            let center_x = (left + right) as f32 / 2.0;
            let center_y = (top + bottom) as f32 / 2.0;
            assert!(
                (center_x - target).abs() <= 2.0 && (center_y - target).abs() <= 2.0,
                "{icon:?} centres at ({center_x}, {center_y}) rather than ({target}, {target})"
            );
        }
    }

    #[test]
    fn a_wide_glyph_keeps_its_aspect_ratio() {
        // Two side-by-side rings are far wider than tall. Normalising must
        // scale them up, never stretch them into a square.
        let glyph = Icon::Ipc.unit_bounds();
        assert!(
            glyph.width() > glyph.height() * 1.4,
            "the ring pair is wide"
        );

        let box_side = 60.0;
        let (left, top, right, bottom) = drawn_extent(Icon::Ipc, box_side);
        let drawn_ratio = (right - left) as f32 / (bottom - top) as f32;
        let authored_ratio = glyph.width() / glyph.height();
        assert!(
            (drawn_ratio - authored_ratio).abs() < 0.35,
            "aspect changed: authored {authored_ratio:.2}, drawn {drawn_ratio:.2}"
        );
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
