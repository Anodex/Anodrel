//! Deterministic synthetic-face tests for text-run construction.

use anodrel_font::{FontFace, FontMetricError};

use crate::{MAX_RUN_GLYPHS, TextRun, TextRunError};

#[test]
fn builds_source_order_glyphs_with_exact_pen_positions() {
    let bytes = metric_face('A', 'B', 1, &[0, 500, 600]);
    let face = FontFace::parse(&bytes).expect("fixture face should parse");
    let run = TextRun::build(&face, "ABA").expect("mapped text should build");

    assert_eq!(run.advance_width(), 1_600);
    assert_eq!(
        run.glyphs()
            .iter()
            .map(|glyph| (glyph.glyph().value(), glyph.pen_x(), glyph.advance_width()))
            .collect::<Vec<_>>(),
        vec![(1, 0, 500), (2, 500, 600), (1, 1_100, 500)]
    );
    assert_eq!(
        (
            run.metrics().units_per_em(),
            run.metrics().ascender(),
            run.metrics().descender(),
            run.metrics().line_gap(),
        ),
        (1_024, 800, -200, 40)
    );
}

#[test]
fn applies_owned_pair_adjustments_before_the_current_glyph_position() {
    let bytes = kerned_metric_face('A', 'B', 1, &[0, 500, 600], &[(1, 2, -80), (2, 1, 20)]);
    let face = FontFace::parse(&bytes).expect("kerning fixture face should parse");
    let run = TextRun::build(&face, "ABA").expect("mapped text should build");

    assert_eq!(run.advance_width(), 1_540);
    assert_eq!(
        run.glyphs()
            .iter()
            .map(|glyph| (glyph.glyph().value(), glyph.pen_x()))
            .collect::<Vec<_>>(),
        vec![(1, 0), (2, 420), (1, 1_040)]
    );
}

#[test]
fn empty_text_still_carries_validated_line_metrics() {
    let bytes = metric_face('A', 'A', 1, &[0, 500]);
    let face = FontFace::parse(&bytes).expect("fixture face should parse");
    let run = TextRun::build(&face, "").expect("empty run should build");

    assert!(run.glyphs().is_empty());
    assert_eq!(run.advance_width(), 0);
    assert_eq!(run.metrics().ascender(), 800);
}

#[test]
fn unavailable_glyph_or_metrics_returns_no_partial_run() {
    let bytes = metric_face('A', 'A', 1, &[0, 500]);
    let face = FontFace::parse(&bytes).expect("fixture face should parse");
    assert_eq!(
        TextRun::build(&face, "AB").unwrap_err(),
        TextRunError::GlyphUnavailable
    );

    let map_only = map_only_face('A', 'A', 1);
    let face = FontFace::parse(&map_only).expect("map-only face should parse");
    assert_eq!(
        TextRun::build(&face, "A").unwrap_err(),
        TextRunError::Metric(FontMetricError::MetricsUnavailable)
    );
}

#[test]
fn rejects_mapped_glyph_outside_the_metric_source() {
    let bytes = metric_face('A', 'A', 2, &[0, 500]);
    let face = FontFace::parse(&bytes).expect("fixture face should parse");

    assert_eq!(
        TextRun::build(&face, "A").unwrap_err(),
        TextRunError::Metric(FontMetricError::InvalidGlyphId)
    );
}

#[test]
fn fixed_scalar_and_advance_limits_refuse_the_complete_request() {
    let bytes = metric_face('A', 'A', 1, &[0, 500]);
    let face = FontFace::parse(&bytes).expect("fixture face should parse");
    assert_eq!(
        TextRun::build(&face, &"A".repeat(MAX_RUN_GLYPHS + 1)).unwrap_err(),
        TextRunError::TooManyGlyphs
    );

    let bytes = metric_face('A', 'A', 1, &[0, u16::MAX]);
    let face = FontFace::parse(&bytes).expect("wide fixture face should parse");
    assert_eq!(
        TextRun::build(&face, &"A".repeat(17)).unwrap_err(),
        TextRunError::AdvanceLimitExceeded
    );

    let bytes = kerned_metric_face('A', 'A', 1, &[0, 0], &[(1, 1, -32_768)]);
    let face = FontFace::parse(&bytes).expect("negative-kerning fixture should parse");
    assert_eq!(
        TextRun::build(&face, &"A".repeat(34)).unwrap_err(),
        TextRunError::AdvanceLimitExceeded
    );
}

fn metric_face(start: char, end: char, first_glyph: u16, advances: &[u16]) -> Vec<u8> {
    sfnt(&[
        (*b"cmap", cmap(start, end, first_glyph)),
        (*b"head", head()),
        (*b"maxp", maximum_profile(advances.len())),
        (*b"hhea", horizontal_header(advances.len())),
        (*b"hmtx", horizontal_metrics(advances)),
    ])
}

fn kerned_metric_face(
    start: char,
    end: char,
    first_glyph: u16,
    advances: &[u16],
    pairs: &[(u16, u16, i16)],
) -> Vec<u8> {
    sfnt(&[
        (*b"cmap", cmap(start, end, first_glyph)),
        (*b"head", head()),
        (*b"maxp", maximum_profile(advances.len())),
        (*b"hhea", horizontal_header(advances.len())),
        (*b"hmtx", horizontal_metrics(advances)),
        (*b"kern", kerning_table(pairs)),
    ])
}

fn map_only_face(start: char, end: char, first_glyph: u16) -> Vec<u8> {
    sfnt(&[(*b"cmap", cmap(start, end, first_glyph))])
}

fn sfnt(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let header_length = 12 + tables.len() * 16;
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 0x0001_0000);
    push_u16(
        &mut bytes,
        u16::try_from(tables.len()).expect("fixture count fits"),
    );
    bytes.extend_from_slice(&[0; 6]);
    let mut offset = header_length;
    for (tag, table) in tables {
        bytes.extend_from_slice(tag);
        push_u32(&mut bytes, 0);
        push_u32(
            &mut bytes,
            u32::try_from(offset).expect("fixture offset fits"),
        );
        push_u32(
            &mut bytes,
            u32::try_from(table.len()).expect("fixture length fits"),
        );
        offset += table.len();
    }
    for (_, table) in tables {
        bytes.extend_from_slice(table);
    }
    bytes
}

fn cmap(start: char, end: char, first_glyph: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 3);
    push_u16(&mut bytes, 1);
    push_u32(&mut bytes, 12);
    push_u16(&mut bytes, 4);
    push_u16(&mut bytes, 32);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 4);
    push_u16(&mut bytes, 4);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, u16::try_from(u32::from(end)).expect("BMP end"));
    push_u16(&mut bytes, u16::MAX);
    push_u16(&mut bytes, 0);
    push_u16(
        &mut bytes,
        u16::try_from(u32::from(start)).expect("BMP start"),
    );
    push_u16(&mut bytes, u16::MAX);
    push_u16(
        &mut bytes,
        first_glyph.wrapping_sub(u16::try_from(u32::from(start)).expect("BMP start")),
    );
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 0);
    bytes
}

fn head() -> Vec<u8> {
    let mut bytes = vec![0; 54];
    bytes[12..16].copy_from_slice(&0x5F0F_3CF5_u32.to_be_bytes());
    bytes[18..20].copy_from_slice(&1_024_u16.to_be_bytes());
    bytes
}

fn maximum_profile(glyph_count: usize) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[0..4].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    bytes[4..6].copy_from_slice(
        &u16::try_from(glyph_count)
            .expect("fixture glyph count fits")
            .to_be_bytes(),
    );
    bytes
}

fn horizontal_header(metric_count: usize) -> Vec<u8> {
    let mut bytes = vec![0; 36];
    bytes[0..4].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    bytes[4..6].copy_from_slice(&800_i16.to_be_bytes());
    bytes[6..8].copy_from_slice(&(-200_i16).to_be_bytes());
    bytes[8..10].copy_from_slice(&40_i16.to_be_bytes());
    bytes[34..36].copy_from_slice(
        &u16::try_from(metric_count)
            .expect("fixture metric count fits")
            .to_be_bytes(),
    );
    bytes
}

fn horizontal_metrics(advances: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(advances.len() * 4);
    for advance in advances {
        push_u16(&mut bytes, *advance);
        bytes.extend_from_slice(&0_i16.to_be_bytes());
    }
    bytes
}

fn kerning_table(pairs: &[(u16, u16, i16)]) -> Vec<u8> {
    let pair_bytes = pairs.len() * 6;
    let search_range = if pairs.is_empty() {
        0
    } else {
        u16::try_from((1_usize << pairs.len().ilog2()) * 6).expect("fixture range fits")
    };
    let mut bytes = Vec::new();
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 0);
    push_u16(
        &mut bytes,
        u16::try_from(14 + pair_bytes).expect("fixture length fits"),
    );
    push_u16(&mut bytes, 1);
    push_u16(
        &mut bytes,
        u16::try_from(pairs.len()).expect("fixture pair count fits"),
    );
    push_u16(&mut bytes, search_range);
    push_u16(
        &mut bytes,
        if pairs.is_empty() {
            0
        } else {
            pairs.len().ilog2() as u16
        },
    );
    push_u16(
        &mut bytes,
        u16::try_from(pair_bytes).expect("fixture pair bytes fit") - search_range,
    );
    for (left, right, adjustment) in pairs {
        push_u16(&mut bytes, *left);
        push_u16(&mut bytes, *right);
        bytes.extend_from_slice(&adjustment.to_be_bytes());
    }
    bytes
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
