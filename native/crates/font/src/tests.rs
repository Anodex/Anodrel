//! Deterministic synthetic face tests.

mod fixtures;

use crate::{FontError, FontFace};
use fixtures::{cmap, format4, format4_with_glyph_array, format12, sfnt};

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
