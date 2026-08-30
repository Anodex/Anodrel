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

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
