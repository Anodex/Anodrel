//! Tiny, in-memory SFNT fixtures with no machine font dependency.

/// Wraps one `cmap` table in a minimal TrueType SFNT face.
pub(super) fn sfnt(character_map: Vec<u8>) -> Vec<u8> {
    sfnt_with_tables(&[(*b"cmap", character_map)])
}

/// Builds one minimal TrueType face from named raw tables.
pub(super) fn sfnt_with_tables(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let header_length = 12 + tables.len() * 16;
    let mut bytes = Vec::with_capacity(
        header_length + tables.iter().map(|(_, table)| table.len()).sum::<usize>(),
    );
    push_u32(&mut bytes, 0x0001_0000);
    push_u16(
        &mut bytes,
        u16::try_from(tables.len()).expect("fixture table count fits"),
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

/// Builds one `cmap` with supplied platform, encoding, and subtable records.
pub(super) fn cmap(records: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
    let header_length = 4 + records.len() * 8;
    let mut bytes = Vec::new();
    push_u16(&mut bytes, 0);
    push_u16(
        &mut bytes,
        u16::try_from(records.len()).expect("fixture record count fits"),
    );
    let mut offset = header_length;
    for (platform, encoding, table) in records {
        push_u16(&mut bytes, *platform);
        push_u16(&mut bytes, *encoding);
        push_u32(
            &mut bytes,
            u32::try_from(offset).expect("fixture offset fits"),
        );
        offset += table.len();
    }
    for (_, _, table) in records {
        bytes.extend_from_slice(table);
    }
    bytes
}

/// Builds a two-segment format-4 map that resolves `A` to one glyph.
pub(super) fn format4(glyph: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u16(&mut bytes, 4);
    push_u16(&mut bytes, 32);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 4);
    push_u16(&mut bytes, 4);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, u16::from(b'A'));
    push_u16(&mut bytes, u16::MAX);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, u16::from(b'A'));
    push_u16(&mut bytes, u16::MAX);
    push_u16(&mut bytes, glyph.wrapping_sub(u16::from(b'A')));
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 0);
    bytes
}

/// Builds a format-4 map that resolves `A` through the glyph-ID array.
pub(super) fn format4_with_glyph_array(glyph: u16) -> Vec<u8> {
    let mut bytes = format4(0);
    bytes[2..4].copy_from_slice(&34_u16.to_be_bytes());
    bytes[24..26].copy_from_slice(&0_u16.to_be_bytes());
    bytes[28..30].copy_from_slice(&4_u16.to_be_bytes());
    push_u16(&mut bytes, glyph);
    bytes
}

/// Builds one format-12 map from sorted groups.
pub(super) fn format12(groups: &[(u32, u32, u32)]) -> Vec<u8> {
    let length = 16 + groups.len() * 12;
    let mut bytes = Vec::new();
    push_u16(&mut bytes, 12);
    push_u16(&mut bytes, 0);
    push_u32(
        &mut bytes,
        u32::try_from(length).expect("fixture length fits"),
    );
    push_u32(&mut bytes, 0);
    push_u32(
        &mut bytes,
        u32::try_from(groups.len()).expect("fixture group count fits"),
    );
    for (start, end, glyph) in groups {
        push_u32(&mut bytes, *start);
        push_u32(&mut bytes, *end);
        push_u32(&mut bytes, *glyph);
    }
    bytes
}

/// Builds a complete face that maps `A` to glyph 1 in one outline table set.
pub(super) fn outline_face(glyph: Vec<u8>, uses_long_locations: bool) -> Vec<u8> {
    outline_face_for_glyph(glyph, uses_long_locations, 1)
}

/// Builds a complete face that maps `A` to one caller-chosen glyph identifier.
pub(super) fn outline_face_for_glyph(
    glyph: Vec<u8>,
    uses_long_locations: bool,
    character_glyph: u16,
) -> Vec<u8> {
    let glyph = pad_to_word(glyph);
    let location_format = if uses_long_locations { 1 } else { 0 };
    sfnt_with_tables(&[
        (*b"cmap", cmap(&[(3, 1, format4(character_glyph))])),
        (*b"head", head(location_format)),
        (*b"maxp", maximum_profile(2)),
        (*b"loca", locations(glyph.len(), uses_long_locations)),
        (*b"glyf", glyph),
    ])
}

/// Builds a metric-only face that maps `A` to one caller-chosen glyph identifier.
pub(super) fn metrics_face(
    long_metrics: &[(u16, i16)],
    trailing_side_bearings: &[i16],
    character_glyph: u16,
) -> Vec<u8> {
    let glyph_count = long_metrics.len() + trailing_side_bearings.len();
    assert!(!long_metrics.is_empty(), "fixture needs one long metric");
    sfnt_with_tables(&[
        (*b"cmap", cmap(&[(3, 1, format4(character_glyph))])),
        (*b"head", head(0)),
        (
            *b"maxp",
            maximum_profile(u16::try_from(glyph_count).expect("glyph count fits")),
        ),
        (*b"hhea", horizontal_header(long_metrics.len())),
        (
            *b"hmtx",
            horizontal_metrics(long_metrics, trailing_side_bearings),
        ),
    ])
}

/// Returns one table record's offset in a synthetic face.
pub(super) fn table_record_offset(face: &[u8], tag: [u8; 4]) -> usize {
    let count = usize::from(u16::from_be_bytes([face[4], face[5]]));
    for index in 0..count {
        let record = 12 + index * 16;
        if face[record..record + 4] == tag {
            return record;
        }
    }
    panic!("fixture table must exist");
}

/// Builds a complete face whose otherwise-valid location index skips glyph-data bytes.
pub(super) fn outline_face_with_nonzero_first_location() -> Vec<u8> {
    let glyph = pad_to_word(simple_triangle());
    let mut location_table = locations(glyph.len(), false);
    location_table[0..2].copy_from_slice(&1_u16.to_be_bytes());
    location_table[2..4].copy_from_slice(&1_u16.to_be_bytes());
    sfnt_with_tables(&[
        (*b"cmap", cmap(&[(3, 1, format4(1))])),
        (*b"head", head(0)),
        (*b"maxp", maximum_profile(2)),
        (*b"loca", location_table),
        (*b"glyf", glyph),
    ])
}

/// Builds one simple triangle whose middle point is off the curve.
pub(super) fn simple_triangle() -> Vec<u8> {
    let mut bytes = glyph_header(1, 0, 0, 20, 20);
    push_u16(&mut bytes, 2);
    push_u16(&mut bytes, 0);
    bytes.extend_from_slice(&[0x31, 0x32, 0x27]);
    bytes.extend_from_slice(&[20, 20]);
    bytes.push(20);
    bytes
}

/// Builds two repeated on-curve points at the origin.
pub(super) fn repeated_zero_points() -> Vec<u8> {
    let mut bytes = glyph_header(1, 0, 0, 0, 0);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 0);
    bytes.extend_from_slice(&[0x39, 1]);
    bytes
}

/// Builds a located zero-contour glyph with its declared design bounds.
pub(super) fn empty_simple_glyph() -> Vec<u8> {
    glyph_header(0, -4, -3, 12, 18)
}

/// Builds a zero-contour glyph whose ignored instructions prove the optional
/// simple-glyph tail is bounded without executing bytecode.
pub(super) fn empty_simple_glyph_with_instruction() -> Vec<u8> {
    let mut glyph = empty_simple_glyph();
    push_u16(&mut glyph, 1);
    glyph.push(0xB0);
    glyph
}

/// Builds a simple glyph whose coordinates use signed 16-bit deltas.
pub(super) fn long_vector_points() -> Vec<u8> {
    let mut bytes = glyph_header(1, 200, -300, 300, -200);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 0);
    bytes.extend_from_slice(&[0x01, 0x01]);
    bytes.extend_from_slice(&300_i16.to_be_bytes());
    bytes.extend_from_slice(&(-100_i16).to_be_bytes());
    bytes.extend_from_slice(&(-300_i16).to_be_bytes());
    bytes.extend_from_slice(&100_i16.to_be_bytes());
    bytes
}

/// Adds ignored instruction bytes ahead of an otherwise valid simple glyph.
pub(super) fn glyph_with_instruction() -> Vec<u8> {
    let mut glyph = simple_triangle();
    glyph[12..14].copy_from_slice(&1_u16.to_be_bytes());
    glyph.insert(14, 0xB0);
    glyph
}

/// Builds only a composite glyph header, which is enough for the closed outcome test.
pub(super) fn composite_glyph() -> Vec<u8> {
    glyph_header(-1, 0, 0, 0, 0)
}

/// Builds an incomplete composite marker with no required glyph header bounds.
pub(super) fn truncated_composite_marker() -> Vec<u8> {
    (-1_i16).to_be_bytes().to_vec()
}

/// Sets the first simple-glyph flag to the reserved bit for malformed-input tests.
pub(super) fn glyph_with_reserved_flag() -> Vec<u8> {
    let mut glyph = simple_triangle();
    glyph[14] |= 0x80;
    glyph
}

/// Appends a non-padding byte after a complete simple glyph description.
pub(super) fn glyph_with_trailing_byte() -> Vec<u8> {
    let mut glyph = simple_triangle();
    glyph.push(0x7F);
    glyph
}

/// Declares more contours than the fixed extraction limit without adding data.
pub(super) fn glyph_over_contour_limit() -> Vec<u8> {
    glyph_header(4_097, 0, 0, 0, 0)
}

fn head(location_format: i16) -> Vec<u8> {
    let mut bytes = vec![0; 54];
    bytes[12..16].copy_from_slice(&0x5F0F_3CF5_u32.to_be_bytes());
    bytes[18..20].copy_from_slice(&1_024_u16.to_be_bytes());
    bytes[50..52].copy_from_slice(&location_format.to_be_bytes());
    bytes
}

fn maximum_profile(glyph_count: u16) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[0..4].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    bytes[4..6].copy_from_slice(&glyph_count.to_be_bytes());
    bytes
}

fn horizontal_header(long_metric_count: usize) -> Vec<u8> {
    let mut bytes = vec![0; 36];
    bytes[0..4].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    bytes[4..6].copy_from_slice(&800_i16.to_be_bytes());
    bytes[6..8].copy_from_slice(&(-200_i16).to_be_bytes());
    bytes[8..10].copy_from_slice(&40_i16.to_be_bytes());
    bytes[34..36].copy_from_slice(
        &u16::try_from(long_metric_count)
            .expect("long metric count fits")
            .to_be_bytes(),
    );
    bytes
}

fn horizontal_metrics(long_metrics: &[(u16, i16)], trailing_side_bearings: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(long_metrics.len() * 4 + trailing_side_bearings.len() * 2);
    for (advance, bearing) in long_metrics {
        push_u16(&mut bytes, *advance);
        bytes.extend_from_slice(&bearing.to_be_bytes());
    }
    for bearing in trailing_side_bearings {
        bytes.extend_from_slice(&bearing.to_be_bytes());
    }
    bytes
}

fn locations(glyph_length: usize, uses_long_locations: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    if uses_long_locations {
        for offset in [0, 0, glyph_length] {
            push_u32(
                &mut bytes,
                u32::try_from(offset).expect("fixture location fits"),
            );
        }
    } else {
        for offset in [0, 0, glyph_length] {
            push_u16(
                &mut bytes,
                u16::try_from(offset / 2).expect("fixture location fits"),
            );
        }
    }
    bytes
}

fn glyph_header(contours: i16, x_min: i16, y_min: i16, x_max: i16, y_max: i16) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [contours, x_min, y_min, x_max, y_max] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    bytes
}

fn pad_to_word(mut bytes: Vec<u8>) -> Vec<u8> {
    if !bytes.len().is_multiple_of(2) {
        bytes.push(0);
    }
    bytes
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
