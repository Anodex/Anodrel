//! One self-contained TrueType face for glyph adapter integration tests.

pub(super) fn simple_outline_face() -> Vec<u8> {
    let glyph = simple_triangle();
    let tables = [
        (*b"cmap", character_map(1)),
        (*b"head", head()),
        (*b"maxp", maximum_profile(2)),
        (*b"hhea", horizontal_header(2)),
        (*b"hmtx", horizontal_metrics(2)),
        (*b"loca", locations(glyph.len())),
        (*b"glyf", glyph),
    ];
    sfnt(tables)
}

pub(super) fn translated_composite_face() -> Vec<u8> {
    let simple = simple_triangle();
    let composite = translated_composite();
    let mut glyphs = simple.clone();
    glyphs.extend_from_slice(&composite);
    let tables = [
        (*b"cmap", character_map(2)),
        (*b"head", head()),
        (*b"maxp", maximum_profile(3)),
        (*b"hhea", horizontal_header(3)),
        (*b"hmtx", horizontal_metrics(3)),
        (*b"loca", composite_locations(simple.len(), composite.len())),
        (*b"glyf", glyphs),
    ];
    sfnt(tables)
}

fn sfnt<const COUNT: usize>(tables: [([u8; 4], Vec<u8>); COUNT]) -> Vec<u8> {
    let header_length = 12 + tables.len() * 16;
    let mut face = Vec::with_capacity(
        header_length + tables.iter().map(|(_, table)| table.len()).sum::<usize>(),
    );
    push_u32(&mut face, 0x0001_0000);
    push_u16(
        &mut face,
        u16::try_from(tables.len()).expect("table count fits"),
    );
    face.extend_from_slice(&[0; 6]);
    let mut offset = header_length;
    for (tag, table) in &tables {
        face.extend_from_slice(tag);
        push_u32(&mut face, 0);
        push_u32(&mut face, u32::try_from(offset).expect("table offset fits"));
        push_u32(
            &mut face,
            u32::try_from(table.len()).expect("table length fits"),
        );
        offset += table.len();
    }
    for (_, table) in tables {
        face.extend_from_slice(&table);
    }
    face
}

fn character_map(glyph: u16) -> Vec<u8> {
    let mut map = Vec::new();
    push_u16(&mut map, 0);
    push_u16(&mut map, 1);
    push_u16(&mut map, 3);
    push_u16(&mut map, 1);
    push_u32(&mut map, 12);
    map.extend_from_slice(&[
        0, 4, 0, 32, 0, 0, 0, 4, 0, 4, 0, 1, 0, 0, 0, b'A', 0xFF, 0xFF, 0, 0, 0, b'A', 0xFF, 0xFF,
        0, 0, 0, 1, 0, 0, 0, 0,
    ]);
    let delta = glyph.wrapping_sub(u16::from(b'A'));
    map[36..38].copy_from_slice(&delta.to_be_bytes());
    map
}

fn head() -> Vec<u8> {
    let mut head = vec![0; 54];
    head[12..16].copy_from_slice(&0x5F0F_3CF5_u32.to_be_bytes());
    head[18..20].copy_from_slice(&1_024_u16.to_be_bytes());
    head
}

fn maximum_profile(glyph_count: u16) -> Vec<u8> {
    let mut profile = vec![0; 32];
    profile[0..4].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    profile[4..6].copy_from_slice(&glyph_count.to_be_bytes());
    profile
}

fn horizontal_header(glyph_count: u16) -> Vec<u8> {
    let mut header = vec![0; 36];
    header[0..4].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    header[4..6].copy_from_slice(&800_i16.to_be_bytes());
    header[6..8].copy_from_slice(&(-200_i16).to_be_bytes());
    header[34..36].copy_from_slice(&glyph_count.to_be_bytes());
    header
}

fn horizontal_metrics(glyph_count: u16) -> Vec<u8> {
    (0..glyph_count)
        .map(|index| if index == 0 { 0_u16 } else { 500_u16 })
        .flat_map(|advance| [advance.to_be_bytes(), 0_i16.to_be_bytes()])
        .flatten()
        .collect()
}

fn locations(glyph_length: usize) -> Vec<u8> {
    let mut locations = Vec::new();
    for offset in [0, 0, glyph_length / 2] {
        push_u16(
            &mut locations,
            u16::try_from(offset).expect("short glyph location fits"),
        );
    }
    locations
}

fn composite_locations(simple_length: usize, composite_length: usize) -> Vec<u8> {
    let mut locations = Vec::new();
    for offset in [
        0,
        0,
        simple_length / 2,
        (simple_length + composite_length) / 2,
    ] {
        push_u16(
            &mut locations,
            u16::try_from(offset).expect("short glyph location fits"),
        );
    }
    locations
}

fn simple_triangle() -> Vec<u8> {
    let mut glyph = Vec::new();
    for value in [1_i16, 0, 0, 20, 20] {
        glyph.extend_from_slice(&value.to_be_bytes());
    }
    push_u16(&mut glyph, 2);
    push_u16(&mut glyph, 0);
    glyph.extend_from_slice(&[0x31, 0x32, 0x27, 20, 20, 20]);
    glyph
}

fn translated_composite() -> Vec<u8> {
    let mut glyph = Vec::new();
    for value in [-1_i16, 0, 0, 50, 20] {
        glyph.extend_from_slice(&value.to_be_bytes());
    }
    for (index, x_offset) in [0_i16, 30].into_iter().enumerate() {
        let flags = 0x0003 | if index == 0 { 0x0020 } else { 0 };
        push_u16(&mut glyph, flags);
        push_u16(&mut glyph, 1);
        glyph.extend_from_slice(&x_offset.to_be_bytes());
        glyph.extend_from_slice(&0_i16.to_be_bytes());
    }
    glyph
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
