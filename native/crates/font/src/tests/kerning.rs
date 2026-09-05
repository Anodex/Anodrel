//! Contract tests for bounded conventional format-0 pair kerning.

use super::{
    FontError, FontFace,
    fixtures::{
        cmap, format4, metrics_face, metrics_face_with_kerning, sfnt_with_tables,
        table_record_offset,
    },
};
use crate::FontKerningError;

type KerningPair = (u16, u16, i16);
type KernSubtable = (u8, Vec<KerningPair>);

#[test]
fn absent_tables_and_unmatched_pairs_are_zero_without_allocating_a_fallback() {
    let absent = metrics_face(&[(0, 0), (500, 0)], &[], 1);
    let face = FontFace::parse(&absent).expect("metric fixture should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(face.horizontal_kerning(glyph, glyph), Ok(0));

    let map_only = sfnt_with_tables(&[(*b"cmap", cmap(&[(3, 1, format4(1))]))]);
    let face = FontFace::parse(&map_only).expect("map-only face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(
        face.horizontal_kerning(glyph, glyph),
        Err(FontKerningError::MetricsUnavailable)
    );

    let present = metrics_face_with_kerning(
        &[(0, 0), (500, 0)],
        &[],
        1,
        kern_table(&[(0x01, vec![(0, 0, -20)])]),
    );
    let face = FontFace::parse(&present).expect("kerning fixture should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(face.horizontal_kerning(glyph, glyph), Ok(0));
}

#[test]
fn sorted_pairs_binary_search_and_override_in_table_order() {
    let bytes = metrics_face_with_kerning(
        &[(0, 0), (500, 0), (600, 0)],
        &[],
        1,
        kern_table(&[
            (0x01, vec![(0, 0, 5), (0, 1, 10), (1, 1, -80)]),
            (0x09, vec![(1, 1, -40)]),
            (0x01, vec![(1, 1, 12)]),
        ]),
    );
    let face = FontFace::parse(&bytes).expect("ordered pair fixture should parse");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(face.horizontal_kerning(glyph, glyph), Ok(-28));
}

#[test]
fn irrelevant_modes_are_ignored_but_malformed_selected_data_refuses_the_face() {
    let ignored = metrics_face_with_kerning(
        &[(0, 0), (500, 0)],
        &[],
        1,
        kern_table(&[(0x00, vec![(1, 1, -80)]), (0x03, vec![(1, 1, -40)])]),
    );
    let face = FontFace::parse(&ignored).expect("vertical and minimum tables are irrelevant");
    let glyph = face.glyph_id('A').expect("fixture maps A");
    assert_eq!(face.horizontal_kerning(glyph, glyph), Ok(0));

    for pairs in [vec![(1, 1, -20), (1, 1, -30)], vec![(2, 1, -20)]] {
        let malformed =
            metrics_face_with_kerning(&[(0, 0), (500, 0)], &[], 1, kern_table(&[(0x01, pairs)]));
        assert_eq!(
            FontFace::parse(&malformed).unwrap_err(),
            FontError::InvalidFace
        );
    }
}

#[test]
fn source_headers_metrics_and_declared_padding_are_checked_before_lookup() {
    let base = metrics_face_with_kerning(
        &[(0, 0), (500, 0)],
        &[],
        1,
        kern_table(&[(0x01, vec![(1, 1, -80)])]),
    );
    let kern_record = table_record_offset(&base, *b"kern");
    let kern_offset = usize::try_from(u32::from_be_bytes(
        base[kern_record + 8..kern_record + 12]
            .try_into()
            .expect("fixture offset is four bytes"),
    ))
    .expect("fixture offset fits");
    for (offset, value) in [
        (kern_offset, 1_u8),
        (kern_offset + 9, 0x11_u8),
        (kern_offset + 13, 5_u8),
        (kern_offset + 15, 1_u8),
        (kern_offset + 17, 1_u8),
    ] {
        let mut malformed = base.clone();
        malformed[offset] = value;
        assert_eq!(
            FontFace::parse(&malformed).unwrap_err(),
            FontError::InvalidFace
        );
    }

    let mut non_padding = base;
    non_padding.push(1);
    let length = u32::try_from(non_padding.len() - kern_offset).expect("fixture length fits");
    non_padding[kern_record + 12..kern_record + 16].copy_from_slice(&length.to_be_bytes());
    assert_eq!(
        FontFace::parse(&non_padding).unwrap_err(),
        FontError::InvalidFace
    );

    let too_many_subtables =
        metrics_face_with_kerning(&[(0, 0), (500, 0)], &[], 1, vec![0, 0, 0, 33]);
    assert_eq!(
        FontFace::parse(&too_many_subtables).unwrap_err(),
        FontError::InvalidFace
    );

    let oversized_source =
        metrics_face_with_kerning(&[(0, 0), (500, 0)], &[], 1, vec![0; 2_097_125]);
    assert_eq!(
        FontFace::parse(&oversized_source).unwrap_err(),
        FontError::InvalidFace
    );
}

#[test]
fn kerning_requires_metrics_and_rejects_an_out_of_range_lookup_id() {
    let missing_metrics = sfnt_with_tables(&[
        (*b"cmap", cmap(&[(3, 1, format4(1))])),
        (*b"kern", kern_table(&[])),
    ]);
    assert_eq!(
        FontFace::parse(&missing_metrics).unwrap_err(),
        FontError::InvalidFace
    );

    let out_of_range = metrics_face_with_kerning(
        &[(0, 0), (500, 0)],
        &[],
        2,
        kern_table(&[(0x01, vec![(1, 1, -80)])]),
    );
    let face = FontFace::parse(&out_of_range).expect("fixture face should parse");
    let glyph = face.glyph_id('A').expect("fixture maps an invalid glyph");
    assert_eq!(
        face.horizontal_kerning(glyph, glyph),
        Err(FontKerningError::InvalidGlyphId)
    );
}

/// Builds a version-0 `kern` table from conventional format-0 subtables.
fn kern_table(subtables: &[KernSubtable]) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u16(&mut bytes, 0);
    push_u16(
        &mut bytes,
        u16::try_from(subtables.len()).expect("fixture subtable count fits"),
    );
    for (flags, pairs) in subtables {
        let pair_bytes = pairs.len() * 6;
        let search = search_range(pairs.len());
        push_u16(&mut bytes, 0);
        push_u16(
            &mut bytes,
            u16::try_from(14 + pair_bytes).expect("fixture subtable length fits"),
        );
        bytes.extend_from_slice(&[0, *flags]);
        push_u16(
            &mut bytes,
            u16::try_from(pairs.len()).expect("fixture pair count fits"),
        );
        push_u16(&mut bytes, search);
        push_u16(&mut bytes, entry_selector(pairs.len()));
        push_u16(
            &mut bytes,
            u16::try_from(pair_bytes).expect("fixture pair bytes fit") - search,
        );
        for (left, right, value) in pairs {
            push_u16(&mut bytes, *left);
            push_u16(&mut bytes, *right);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
    }
    bytes
}

/// Returns the format-0 search range for a fixture pair count.
fn search_range(pair_count: usize) -> u16 {
    if pair_count == 0 {
        return 0;
    }
    u16::try_from((1_usize << pair_count.ilog2()) * 6).expect("fixture range fits")
}

/// Returns the format-0 selector for a fixture pair count.
fn entry_selector(pair_count: usize) -> u16 {
    if pair_count == 0 {
        0
    } else {
        pair_count.ilog2() as u16
    }
}

/// Appends one big-endian unsigned 16-bit fixture value.
fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
