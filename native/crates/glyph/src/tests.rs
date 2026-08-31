//! Deterministic tests for the bounded glyph-to-canvas adapter.

mod fixture;

use anodrel_canvas::point;
use anodrel_font::FontFace;

use crate::flatten::{Quadratic, append_quadratic};
use crate::{GlyphPlacement, GlyphRenderError, canvas_path, coverage_mask};

#[test]
fn placement_flips_the_font_vertical_axis_at_its_baseline() {
    let placement = GlyphPlacement::new(point(10.0, 20.0), 2.0).expect("placement should fit");
    assert_eq!(placement.map_doubled(6, 4), point(16.0, 16.0));
}

#[test]
fn placement_refuses_nonfinite_and_out_of_range_values() {
    assert_eq!(
        GlyphPlacement::new(point(f32::NAN, 0.0), 1.0),
        Err(GlyphRenderError::InvalidPlacement)
    );
    assert_eq!(
        GlyphPlacement::new(point(0.0, 0.0), 0.0),
        Err(GlyphRenderError::InvalidPlacement)
    );
    assert_eq!(
        GlyphPlacement::new(point(0.0, 0.0), 64.1),
        Err(GlyphRenderError::InvalidPlacement)
    );
}

#[test]
fn flat_quadratics_emit_one_chord_endpoint() {
    let mut points = vec![point(0.0, 0.0)];
    let mut vertex_count = 1;
    append_quadratic(
        Quadratic::new(point(0.0, 0.0), point(5.0, 0.0), point(10.0, 0.0)),
        &mut points,
        &mut vertex_count,
    )
    .expect("flat curve should flatten");
    assert_eq!(points, vec![point(0.0, 0.0), point(10.0, 0.0)]);
}

#[test]
fn curved_quadratics_subdivide_in_order() {
    let mut points = vec![point(0.0, 0.0)];
    let mut vertex_count = 1;
    append_quadratic(
        Quadratic::new(point(0.0, 0.0), point(0.0, 12.0), point(12.0, 12.0)),
        &mut points,
        &mut vertex_count,
    )
    .expect("bounded curve should flatten");
    assert!(points.len() > 2);
    assert!(points.windows(2).all(|pair| pair[0].x <= pair[1].x));
    assert_eq!(points.last(), Some(&point(12.0, 12.0)));
}

#[test]
fn curves_that_exceed_the_quality_depth_are_refused() {
    let mut points = vec![point(0.0, 0.0)];
    let mut vertex_count = 1;
    let result = append_quadratic(
        Quadratic::new(
            point(0.0, 0.0),
            point(0.0, 4_000_000.0),
            point(4_000_000.0, 4_000_000.0),
        ),
        &mut points,
        &mut vertex_count,
    );
    assert_eq!(result, Err(GlyphRenderError::TooComplex));
}

#[test]
fn parsed_quadratic_paths_flatten_into_one_open_canvas_contour() {
    let bytes = fixture::simple_outline_face();
    let face = FontFace::parse(&bytes).expect("synthetic face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    let outline = face
        .glyph_outline(glyph)
        .expect("fixture has a simple glyph");
    let glyph_path = outline.quadratic_path();
    let placement = GlyphPlacement::new(point(10.0, 30.0), 1.0).expect("placement fits");
    let flattened = canvas_path(&glyph_path, placement).expect("glyph should flatten");
    assert_eq!(flattened.contours().len(), 1);
    assert!(flattened.contours()[0].len() > 3);
    assert_eq!(flattened.contours()[0].first(), Some(&point(10.0, 30.0)));
    assert_ne!(
        flattened.contours()[0].first(),
        flattened.contours()[0].last()
    );
}

#[test]
fn parsed_glyphs_rasterize_to_a_bounded_coverage_mask() {
    let bytes = fixture::simple_outline_face();
    let face = FontFace::parse(&bytes).expect("synthetic face should parse");
    let outline = face
        .glyph_outline(face.glyph_id('A').expect("fixture maps A"))
        .expect("fixture has a simple glyph");
    let glyph_path = outline.quadratic_path();
    let placement = GlyphPlacement::new(point(10.0, 30.0), 1.0).expect("placement fits");
    let mask = coverage_mask(&glyph_path, placement).expect("small glyph fits the mask limit");
    assert!(mask.width() > 0 && mask.height() > 0);
    assert!((0..mask.height()).any(|y| (0..mask.width()).any(|x| mask.coverage_at(x, y) > 0.0)));
}

#[test]
fn glyph_coverage_refuses_an_oversized_transformed_path() {
    let bytes = fixture::simple_outline_face();
    let face = FontFace::parse(&bytes).expect("synthetic face should parse");
    let outline = face
        .glyph_outline(face.glyph_id('A').expect("fixture maps A"))
        .expect("fixture has a simple glyph");
    let placement = GlyphPlacement::new(point(0.0, 0.0), 64.0).expect("maximum scale fits");
    assert!(matches!(
        coverage_mask(&outline.quadratic_path(), placement),
        Err(GlyphRenderError::TooComplex)
    ));
}
