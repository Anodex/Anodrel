//! Exact quadratic-path conversion tests.

use crate::{GlyphBounds, GlyphOutline, GlyphPathPoint, GlyphPoint, GlyphSegment};

#[test]
fn explicit_and_off_curve_points_produce_closed_line_and_quadratic_segments() {
    let outline = outline(&[&[(0, 0, true), (20, 0, false), (0, 20, true)]]);
    let path = outline.quadratic_path();
    assert_eq!(path.contour_count(), 1);
    assert_eq!(path.segment_count(), 2);
    assert_eq!(path.contour_start(0), Some(point(0, 0)));
    assert_eq!(
        path.segment_slice(0),
        Some(
            [
                GlyphSegment::QuadraticTo {
                    control: point(40, 0),
                    to: point(0, 40),
                },
                GlyphSegment::LineTo { to: point(0, 0) },
            ]
            .as_slice()
        )
    );
}

#[test]
fn consecutive_off_curve_controls_gain_an_exact_implied_midpoint() {
    let outline = outline(&[&[(0, 0, true), (4, 0, false), (8, 0, false), (8, 8, true)]]);
    let path = outline.quadratic_path();
    assert_eq!(
        path.segment_slice(0),
        Some(
            [
                GlyphSegment::QuadraticTo {
                    control: point(8, 0),
                    to: point(12, 0),
                },
                GlyphSegment::QuadraticTo {
                    control: point(16, 0),
                    to: point(16, 16),
                },
                GlyphSegment::LineTo { to: point(0, 0) },
            ]
            .as_slice()
        )
    );
}

#[test]
fn off_curve_start_uses_the_final_on_curve_point_as_its_start() {
    let outline = outline(&[&[(2, 0, false), (4, 0, true), (4, 4, true)]]);
    let path = outline.quadratic_path();
    assert_eq!(path.contour_start(0), Some(point(8, 8)));
    assert_eq!(
        path.segment_slice(0),
        Some(
            [
                GlyphSegment::QuadraticTo {
                    control: point(4, 0),
                    to: point(8, 0),
                },
                GlyphSegment::LineTo { to: point(8, 8) },
            ]
            .as_slice()
        )
    );
}

#[test]
fn all_off_curve_contour_starts_at_the_exact_boundary_midpoint() {
    let outline = outline(&[&[(2, 0, false), (4, 0, false)]]);
    let path = outline.quadratic_path();
    assert_eq!(path.contour_start(0), Some(point(6, 0)));
    assert_eq!(
        path.segment_slice(0),
        Some(
            [
                GlyphSegment::QuadraticTo {
                    control: point(4, 0),
                    to: point(6, 0),
                },
                GlyphSegment::QuadraticTo {
                    control: point(8, 0),
                    to: point(6, 0),
                },
            ]
            .as_slice()
        )
    );
}

#[test]
fn duplicate_coordinates_keep_their_source_curve_state() {
    let outline = outline(&[&[(0, 0, true), (4, 0, false), (4, 0, true)]]);
    let path = outline.quadratic_path();
    assert!(matches!(
        path.segment_slice(0),
        Some([
            GlyphSegment::QuadraticTo { .. },
            GlyphSegment::LineTo { .. }
        ])
    ));
}

#[test]
fn empty_outline_converts_to_an_empty_path() {
    let path = GlyphOutline::empty().quadratic_path();
    assert_eq!(path.contour_count(), 0);
    assert_eq!(path.segment_count(), 0);
    assert_eq!(path.contour_start(0), None);
    assert_eq!(path.segment_slice(0), None);
}

fn outline(contours: &[&[(i16, i16, bool)]]) -> GlyphOutline {
    let mut points = Vec::new();
    let mut ends = Vec::new();
    for contour in contours {
        points.extend(
            contour
                .iter()
                .map(|(x, y, on_curve)| GlyphPoint::new(*x, *y, *on_curve)),
        );
        ends.push(points.len() - 1);
    }
    GlyphOutline::new(
        GlyphBounds::new(0, 0, 0, 0).expect("test bounds are ordered"),
        points,
        ends,
    )
}

fn point(x_twice: i32, y_twice: i32) -> GlyphPathPoint {
    GlyphPathPoint::new(x_twice, y_twice)
}
