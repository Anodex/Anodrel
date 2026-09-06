//! Contract tests for bounded basic-Latin GPOS pair positioning.

use crate::{FontError, FontFace};

use super::fixtures::metrics_face_with_gpos;

#[test]
fn format_one_gpos_kern_replaces_the_legacy_fallback_for_basic_latin() {
    let bytes = metrics_face_with_gpos(&[(0, 0), (500, 0)], &[], 1, format_one(-80));
    let face = FontFace::parse(&bytes).expect("format-one GPOS face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(face.horizontal_kerning(glyph, glyph), Ok(0));
    assert_eq!(face.basic_latin_horizontal_kerning(glyph, glyph), Ok(-80));
}

#[test]
fn format_two_gpos_kern_uses_validated_class_records() {
    let bytes = metrics_face_with_gpos(&[(0, 0), (500, 0)], &[], 1, format_two(-120));
    let face = FontFace::parse(&bytes).expect("format-two GPOS face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(face.basic_latin_horizontal_kerning(glyph, glyph), Ok(-120));
}

#[test]
fn extension_pair_positioning_with_ignore_marks_is_selected_for_basic_latin() {
    let bytes = metrics_face_with_gpos(&[(0, 0), (500, 0)], &[], 1, extension_format_one(-96));
    let face = FontFace::parse(&bytes).expect("extension GPOS face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(face.basic_latin_horizontal_kerning(glyph, glyph), Ok(-96));
}

#[test]
fn unsupported_value_records_leave_the_basic_latin_lookup_at_zero() {
    let mut table = format_one(-80);
    let lookup = lookup_offset(&table);
    let subtable = lookup + 12;
    table[subtable + 4..subtable + 6].copy_from_slice(&1_u16.to_be_bytes());
    let bytes = metrics_face_with_gpos(&[(0, 0), (500, 0)], &[], 1, table);
    let face = FontFace::parse(&bytes).expect("unsupported valid value record is ignored");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(face.basic_latin_horizontal_kerning(glyph, glyph), Ok(0));
}

#[test]
fn malformed_selected_gpos_offsets_refuse_the_face() {
    let mut table = format_one(-80);
    table[4..6].copy_from_slice(&u16::MAX.to_be_bytes());
    let bytes = metrics_face_with_gpos(&[(0, 0), (500, 0)], &[], 1, table);
    assert_eq!(FontFace::parse(&bytes).unwrap_err(), FontError::InvalidFace);
}

/// Builds a complete GPOS table with one format-one x-advance pair subtable.
fn format_one(adjustment: i16) -> Vec<u8> {
    let mut pair = Vec::new();
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 18);
    push_u16(&mut pair, 0x0004);
    push_u16(&mut pair, 0);
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 12);
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 1);
    push_i16(&mut pair, adjustment);
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 1);
    gpos(pair)
}

/// Builds a complete GPOS table with one format-two x-advance class subtable.
fn format_two(adjustment: i16) -> Vec<u8> {
    let mut pair = Vec::new();
    push_u16(&mut pair, 2);
    push_u16(&mut pair, 24);
    push_u16(&mut pair, 0x0004);
    push_u16(&mut pair, 0);
    push_u16(&mut pair, 30);
    push_u16(&mut pair, 38);
    push_u16(&mut pair, 2);
    push_u16(&mut pair, 2);
    for value in [0, 0, 0, adjustment] {
        push_i16(&mut pair, value);
    }
    // Coverage format one: one first glyph at coverage index zero.
    for value in [1, 1, 1] {
        push_u16(&mut pair, value);
    }
    // Two format-one class definitions: glyph one belongs to class one.
    for value in [1, 1, 1, 1, 1, 1, 1, 1] {
        push_u16(&mut pair, value);
    }
    gpos(pair)
}

/// Wraps one pair-positioning subtable in `latn`/default/`kern` GPOS tables.
fn gpos(pair: Vec<u8>) -> Vec<u8> {
    gpos_with_lookup(pair, 2, 0)
}

/// Wraps a pair subtable in the standard type-nine extension form.
fn extension_format_one(adjustment: i16) -> Vec<u8> {
    let pair = format_one_pair(adjustment);
    let mut extension = Vec::new();
    push_u16(&mut extension, 1);
    push_u16(&mut extension, 2);
    push_u32(&mut extension, 8);
    extension.extend_from_slice(&pair);
    gpos_with_lookup(extension, 9, 0x0008)
}

/// Wraps one positioning subtable in `latn`/default/`kern` GPOS tables.
fn gpos_with_lookup(subtable: Vec<u8>, lookup_type: u16, flags: u16) -> Vec<u8> {
    let script = script_list();
    let feature = feature_list();
    let mut lookup = Vec::new();
    push_u16(&mut lookup, 1);
    push_u16(&mut lookup, 4);
    push_u16(&mut lookup, lookup_type);
    push_u16(&mut lookup, flags);
    push_u16(&mut lookup, 1);
    push_u16(&mut lookup, 8);
    lookup.extend_from_slice(&subtable);

    let script_offset = 10_u16;
    let feature_offset = script_offset + u16::try_from(script.len()).expect("fixture fits");
    let lookup_offset = feature_offset + u16::try_from(feature.len()).expect("fixture fits");
    let mut result = Vec::new();
    push_u16(&mut result, 1);
    push_u16(&mut result, 0);
    push_u16(&mut result, script_offset);
    push_u16(&mut result, feature_offset);
    push_u16(&mut result, lookup_offset);
    result.extend_from_slice(&script);
    result.extend_from_slice(&feature);
    result.extend_from_slice(&lookup);
    result
}

/// Builds only the format-one pair subtable used by direct and extension tests.
fn format_one_pair(adjustment: i16) -> Vec<u8> {
    let mut pair = Vec::new();
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 18);
    push_u16(&mut pair, 0x0004);
    push_u16(&mut pair, 0);
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 12);
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 1);
    push_i16(&mut pair, adjustment);
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 1);
    push_u16(&mut pair, 1);
    pair
}

/// Builds the exact `latn` script record with one default language system.
fn script_list() -> Vec<u8> {
    let mut result = Vec::new();
    push_u16(&mut result, 1);
    result.extend_from_slice(b"latn");
    push_u16(&mut result, 8);
    push_u16(&mut result, 4);
    push_u16(&mut result, 0);
    push_u16(&mut result, 0);
    push_u16(&mut result, u16::MAX);
    push_u16(&mut result, 1);
    push_u16(&mut result, 0);
    result
}

/// Builds one `kern` feature that selects lookup zero.
fn feature_list() -> Vec<u8> {
    let mut result = Vec::new();
    push_u16(&mut result, 1);
    result.extend_from_slice(b"kern");
    push_u16(&mut result, 8);
    push_u16(&mut result, 0);
    push_u16(&mut result, 1);
    push_u16(&mut result, 0);
    result
}

/// Resolves the fixture's first lookup offset from the GPOS header.
fn lookup_offset(table: &[u8]) -> usize {
    usize::from(u16::from_be_bytes([table[8], table[9]]))
}

/// Appends one big-endian unsigned 16-bit fixture value.
fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

/// Appends one big-endian unsigned 32-bit fixture value.
fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

/// Appends one big-endian signed 16-bit fixture value.
fn push_i16(bytes: &mut Vec<u8>, value: i16) {
    push_u16(bytes, value as u16);
}
