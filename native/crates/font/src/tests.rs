//! Deterministic synthetic face tests.

mod fixtures;

use crate::{FontError, FontFace, GlyphOutlineError};
use fixtures::{
    cmap, composite_glyph, format4, format4_with_glyph_array, format12, glyph_over_contour_limit,
    glyph_with_instruction, glyph_with_reserved_flag, glyph_with_trailing_byte, long_vector_points,
    outline_face, outline_face_for_glyph, outline_face_with_nonzero_first_location,
    repeated_zero_points, sfnt, simple_triangle, truncated_composite_marker,
};

#[test]
fn format_four_maps_bmp_character_without_copying_face_bytes() {
    let bytes = sfnt(cmap(&[(3, 1, format4(5))]));
    let face = FontFace::parse(&bytes).expect("format four face should parse");
    assert_eq!(face.glyph_id('A').map(|glyph| glyph.value()), Some(5));
    assert_eq!(face.glyph_id('B'), None);
    assert_eq!(face.glyph_id('😀'), None);
}

#[test]
fn format_twelve_maps_non_bmp_character() {
    let bytes = sfnt(cmap(&[(0, 4, format12(&[(0x1F_600, 0x1F_600, 77)]))]));
    let face = FontFace::parse(&bytes).expect("format twelve face should parse");
    assert_eq!(face.glyph_id('😀').map(|glyph| glyph.value()), Some(77));
    assert_eq!(face.glyph_id('A'), None);
}

#[test]
fn unicode_bmp_map_resolves_a_basic_character() {
    let bytes = sfnt(cmap(&[(0, 3, format4(8))]));
    let face = FontFace::parse(&bytes).expect("Unicode BMP face should parse");
    assert_eq!(face.glyph_id('A').map(|glyph| glyph.value()), Some(8));
}

#[test]
fn format_four_maps_through_its_glyph_array() {
    let bytes = sfnt(cmap(&[(3, 1, format4_with_glyph_array(9))]));
    let face = FontFace::parse(&bytes).expect("glyph-array face should parse");
    assert_eq!(face.glyph_id('A').map(|glyph| glyph.value()), Some(9));
}

#[test]
fn selected_map_prefers_windows_full_unicode_over_bmp() {
    let bytes = sfnt(cmap(&[
        (3, 1, format4(5)),
        (3, 10, format12(&[(u32::from('A'), u32::from('A'), 7)])),
    ]));
    let face = FontFace::parse(&bytes).expect("priority face should parse");
    assert_eq!(face.glyph_id('A').map(|glyph| glyph.value()), Some(7));
}

#[test]
fn zero_glyph_is_not_reported_as_content() {
    let bytes = sfnt(cmap(&[(
        3,
        10,
        format12(&[(u32::from('A'), u32::from('A'), 0)]),
    )]));
    let face = FontFace::parse(&bytes).expect("zero glyph map should parse");
    assert_eq!(face.glyph_id('A'), None);
}

#[test]
fn malformed_directory_range_is_rejected() {
    let mut bytes = sfnt(cmap(&[(3, 1, format4(5))]));
    bytes[24..28].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(FontFace::parse(&bytes).unwrap_err(), FontError::InvalidFace);
}

#[test]
fn malformed_encoding_offset_is_rejected() {
    let mut bytes = sfnt(cmap(&[(3, 1, format4(5))]));
    let cmap_offset = 28;
    bytes[cmap_offset + 8..cmap_offset + 12].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        FontFace::parse(&bytes).unwrap_err(),
        FontError::InvalidCharacterMap
    );
}

#[test]
fn backward_format_four_glyph_offset_is_rejected() {
    let mut map = format4(5);
    map[28..30].copy_from_slice(&2_u16.to_be_bytes());
    let bytes = sfnt(cmap(&[(3, 1, map)]));
    assert_eq!(
        FontFace::parse(&bytes).unwrap_err(),
        FontError::InvalidCharacterMap
    );
}

#[test]
fn unordered_format_four_segments_are_rejected() {
    let mut map = format4(5);
    map[20..22].copy_from_slice(&u16::from(b'B').to_be_bytes());
    let bytes = sfnt(cmap(&[(3, 1, map)]));
    assert_eq!(
        FontFace::parse(&bytes).unwrap_err(),
        FontError::InvalidCharacterMap
    );
}

#[test]
fn unordered_format_twelve_groups_are_rejected() {
    let bytes = sfnt(cmap(&[(
        3,
        10,
        format12(&[(0x100, 0x100, 2), (0x80, 0x80, 3)]),
    )]));
    assert_eq!(
        FontFace::parse(&bytes).unwrap_err(),
        FontError::InvalidCharacterMap
    );
}

#[test]
fn overflowing_format_twelve_glyph_range_is_rejected() {
    let bytes = sfnt(cmap(&[(
        3,
        10,
        format12(&[(0x41, 0x42, u32::from(u16::MAX))]),
    )]));
    assert_eq!(
        FontFace::parse(&bytes).unwrap_err(),
        FontError::InvalidCharacterMap
    );
}

#[test]
fn duplicate_character_map_tables_are_rejected() {
    let bytes = fixtures::sfnt_with_tables(&[
        (*b"cmap", cmap(&[(3, 1, format4(5))])),
        (*b"cmap", cmap(&[(3, 1, format4(6))])),
    ]);
    assert_eq!(FontFace::parse(&bytes).unwrap_err(), FontError::InvalidFace);
}

#[test]
fn face_without_a_character_map_is_rejected() {
    let bytes = fixtures::sfnt_with_tables(&[(*b"head", Vec::new())]);
    assert_eq!(
        FontFace::parse(&bytes).unwrap_err(),
        FontError::MissingCharacterMap
    );
}

#[test]
fn unsupported_face_and_map_are_closed_errors() {
    assert_eq!(FontFace::parse(&[]).unwrap_err(), FontError::InvalidFace);
    let bytes = sfnt(cmap(&[(3, 1, vec![0, 0, 0, 0])]));
    assert_eq!(
        FontFace::parse(&bytes).unwrap_err(),
        FontError::UnsupportedCharacterMap
    );
}

#[test]
fn simple_outline_preserves_points_contours_bounds_and_curve_state() {
    let bytes = outline_face(simple_triangle(), false);
    let face = FontFace::parse(&bytes).expect("complete outline face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    let outline = face
        .glyph_outline(glyph)
        .expect("simple outline should parse");
    assert_eq!(outline.bounds().x_max(), 20);
    assert_eq!(outline.bounds().y_max(), 20);
    assert_eq!(outline.contour_count(), 1);
    assert_eq!(outline.point_count(), 3);
    let contour = outline.point_slice(0).expect("first contour exists");
    assert_eq!((contour[0].x(), contour[0].y()), (0, 0));
    assert_eq!((contour[1].x(), contour[1].y()), (20, 0));
    assert_eq!((contour[2].x(), contour[2].y()), (0, 20));
    assert!(contour[0].is_on_curve());
    assert!(!contour[1].is_on_curve());
    assert!(contour[2].is_on_curve());
    assert_eq!(outline.point_slice(1), None);
}

#[test]
fn long_locations_extract_the_same_simple_outline() {
    let bytes = outline_face(simple_triangle(), true);
    let face = FontFace::parse(&bytes).expect("long-location face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    let outline = face
        .glyph_outline(glyph)
        .expect("long location resolves glyph");
    assert_eq!(outline.point_count(), 3);
}

#[test]
fn packed_repeated_flags_expand_without_coordinate_bytes() {
    let bytes = outline_face(repeated_zero_points(), false);
    let face = FontFace::parse(&bytes).expect("repeated flag face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    let outline = face
        .glyph_outline(glyph)
        .expect("repeated flags should expand");
    assert_eq!(outline.point_count(), 2);
    assert!(
        outline
            .point_slice(0)
            .expect("first contour exists")
            .iter()
            .all(|point| point.x() == 0 && point.y() == 0 && point.is_on_curve())
    );
}

#[test]
fn signed_long_coordinate_vectors_accumulate_in_design_units() {
    let bytes = outline_face(long_vector_points(), false);
    let face = FontFace::parse(&bytes).expect("long-vector face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    let outline = face
        .glyph_outline(glyph)
        .expect("long vectors should parse");
    let contour = outline.point_slice(0).expect("first contour exists");
    assert_eq!((contour[0].x(), contour[0].y()), (300, -300));
    assert_eq!((contour[1].x(), contour[1].y()), (200, -200));
}

#[test]
fn instruction_bytes_are_skipped_without_execution() {
    let bytes = outline_face(glyph_with_instruction(), false);
    let face = FontFace::parse(&bytes).expect("instruction face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(
        face.glyph_outline(glyph)
            .expect("instruction bytes are skipped")
            .point_count(),
        3
    );
}

#[test]
fn empty_located_glyph_is_distinct_from_missing_outline_source() {
    let bytes = outline_face(Vec::new(), false);
    let face = FontFace::parse(&bytes).expect("empty outline face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    let outline = face.glyph_outline(glyph).expect("empty glyph is valid");
    assert_eq!(outline.contour_count(), 0);
    assert_eq!(outline.point_count(), 0);

    let map_only_bytes = sfnt(cmap(&[(3, 1, format4(1))]));
    let map_only = FontFace::parse(&map_only_bytes).expect("map-only face should parse");
    let glyph = map_only.glyph_id('A').expect("fixture maps A");
    assert_eq!(
        map_only.glyph_outline(glyph).unwrap_err(),
        GlyphOutlineError::OutlineUnavailable
    );
}

#[test]
fn composite_and_reserved_simple_glyphs_are_refused() {
    for (glyph, expected) in [
        (
            composite_glyph(),
            GlyphOutlineError::CompositeGlyphUnsupported,
        ),
        (
            glyph_with_reserved_flag(),
            GlyphOutlineError::MalformedOutline,
        ),
        (
            glyph_with_trailing_byte(),
            GlyphOutlineError::MalformedOutline,
        ),
        (
            glyph_over_contour_limit(),
            GlyphOutlineError::ComplexityLimitExceeded,
        ),
        (
            truncated_composite_marker(),
            GlyphOutlineError::MalformedOutline,
        ),
    ] {
        let bytes = outline_face(glyph, false);
        let face = FontFace::parse(&bytes).expect("outline-table source should parse");
        let glyph = face.glyph_id('A').expect("fixture maps A");
        assert_eq!(face.glyph_outline(glyph).unwrap_err(), expected);
    }
}

#[test]
fn partial_outline_tables_are_rejected_during_face_parsing() {
    let bytes = fixtures::sfnt_with_tables(&[
        (*b"cmap", cmap(&[(3, 1, format4(1))])),
        (*b"head", vec![0; 54]),
    ]);
    assert_eq!(FontFace::parse(&bytes).unwrap_err(), FontError::InvalidFace);
}

#[test]
fn location_index_must_begin_at_the_glyph_data_start() {
    let bytes = outline_face_with_nonzero_first_location();
    assert_eq!(FontFace::parse(&bytes).unwrap_err(), FontError::InvalidFace);
}

#[test]
fn mapped_glyph_outside_maximum_profile_is_refused() {
    let bytes = outline_face_for_glyph(simple_triangle(), false, 2);
    let face = FontFace::parse(&bytes).expect("face tables are valid");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(
        face.glyph_outline(glyph).unwrap_err(),
        GlyphOutlineError::InvalidGlyphId
    );
}
