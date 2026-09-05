//! Contract tests for the bounded translated-composite glyph subset.

use super::{
    FontFace, GlyphOutlineError,
    fixtures::{composite_glyph, outline_face_with_glyphs, simple_triangle, translated_composite},
};

#[test]
fn translated_components_flatten_into_one_normal_outline() {
    let composite = translated_composite((0, 0, 50, 20), &[(1, 0, 0), (1, 30, 0)]);
    let bytes = outline_face_with_glyphs(&[Vec::new(), simple_triangle(), composite], false, 2);
    let face = FontFace::parse(&bytes).expect("composite face should parse");
    let outline = face
        .glyph_outline(face.glyph_id('A').expect("fixture maps A"))
        .expect("translated composite should flatten");
    assert_eq!(outline.contour_count(), 2);
    assert_eq!(outline.point_count(), 6);
    assert_eq!(
        (
            outline.bounds().x_min(),
            outline.bounds().y_min(),
            outline.bounds().x_max(),
            outline.bounds().y_max(),
        ),
        (0, 0, 50, 20)
    );
    assert_eq!(
        outline
            .point_slice(1)
            .expect("second component contour exists")[0]
            .x(),
        30
    );
}

#[test]
fn nested_translation_and_composite_cycles_have_closed_outcomes() {
    let child = translated_composite((0, 0, 20, 20), &[(1, 4, 6)]);
    let parent = translated_composite((0, 0, 30, 30), &[(2, 7, 8)]);
    let bytes = outline_face_with_glyphs(&[Vec::new(), simple_triangle(), child, parent], false, 3);
    let face = FontFace::parse(&bytes).expect("nested face should parse");
    let outline = face
        .glyph_outline(face.glyph_id('A').expect("fixture maps A"))
        .expect("nested translation should flatten");
    assert_eq!(
        outline.point_slice(0).expect("nested contour exists")[0].x(),
        11
    );
    assert_eq!(
        outline.point_slice(0).expect("nested contour exists")[0].y(),
        14
    );

    let first = translated_composite((0, 0, 20, 20), &[(2, 0, 0)]);
    let second = translated_composite((0, 0, 20, 20), &[(1, 0, 0)]);
    let bytes = outline_face_with_glyphs(&[Vec::new(), first, second], false, 1);
    let face = FontFace::parse(&bytes).expect("cyclic source still parses its tables");
    assert_eq!(
        face.glyph_outline(face.glyph_id('A').expect("fixture maps A"))
            .unwrap_err(),
        GlyphOutlineError::CompositeCycle
    );
}

#[test]
fn unsupported_composite_operations_never_round_or_partially_flatten() {
    let mut transformed = translated_composite((0, 0, 20, 20), &[(1, 0, 0)]);
    transformed[10..12].copy_from_slice(&0x000B_u16.to_be_bytes());
    let mut point_attached = translated_composite((0, 0, 40, 20), &[(1, 0, 0), (1, 0, 0)]);
    point_attached[18..20].copy_from_slice(&0x0001_u16.to_be_bytes());
    let out_of_range = translated_composite((0, 0, i16::MAX, 20), &[(1, i16::MAX, 0)]);
    for (glyph, expected) in [
        (
            transformed,
            GlyphOutlineError::CompositeTransformUnsupported,
        ),
        (
            point_attached,
            GlyphOutlineError::CompositePointAttachmentUnsupported,
        ),
        (
            out_of_range,
            GlyphOutlineError::CompositeCoordinateOutOfRange,
        ),
        (composite_glyph(), GlyphOutlineError::MalformedOutline),
    ] {
        let bytes = outline_face_with_glyphs(&[Vec::new(), simple_triangle(), glyph], false, 2);
        let face = FontFace::parse(&bytes).expect("composite source tables should parse");
        assert_eq!(
            face.glyph_outline(face.glyph_id('A').expect("fixture maps A"))
                .unwrap_err(),
            expected
        );
    }
}

#[test]
fn composite_instructions_are_bounded_but_never_executed() {
    let mut composite = translated_composite((0, 0, 20, 20), &[(1, 0, 0)]);
    composite[10..12].copy_from_slice(&0x0103_u16.to_be_bytes());
    composite.extend_from_slice(&[0, 1, 0xB0]);
    let bytes = outline_face_with_glyphs(&[Vec::new(), simple_triangle(), composite], false, 2);
    let face = FontFace::parse(&bytes).expect("instruction-bearing composite face should parse");
    assert_eq!(
        face.glyph_outline(face.glyph_id('A').expect("fixture maps A"))
            .expect("bounded instructions are ignored")
            .point_count(),
        3
    );
}

#[test]
fn composite_component_and_nesting_limits_fail_closed() {
    let too_many_components = translated_composite((0, 0, 20, 20), &vec![(1, 0, 0); 129]);
    let bytes = outline_face_with_glyphs(
        &[Vec::new(), simple_triangle(), too_many_components],
        false,
        2,
    );
    let face = FontFace::parse(&bytes).expect("over-limit component source tables should parse");
    assert_eq!(
        face.glyph_outline(face.glyph_id('A').expect("fixture maps A"))
            .unwrap_err(),
        GlyphOutlineError::ComplexityLimitExceeded
    );

    let mut glyphs = vec![Vec::new(), simple_triangle()];
    for component in 1..=17_u16 {
        glyphs.push(translated_composite((0, 0, 20, 20), &[(component, 0, 0)]));
    }
    let bytes = outline_face_with_glyphs(&glyphs, false, 18);
    let face = FontFace::parse(&bytes).expect("over-depth source tables should parse");
    assert_eq!(
        face.glyph_outline(face.glyph_id('A').expect("fixture maps A"))
            .unwrap_err(),
        GlyphOutlineError::ComplexityLimitExceeded
    );
}
