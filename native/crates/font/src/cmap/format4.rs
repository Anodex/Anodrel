//! Validated format-4 Basic Multilingual Plane segment maps.

use crate::{FontError, GlyphId, bytes::Bytes};

const HEADER_LENGTH: usize = 14;
const SEGMENT_WORD_LENGTH: usize = 2;

/// One validated format-4 map borrowed from the face bytes.
pub(crate) struct Format4<'font> {
    table: Bytes<'font>,
    segment_count: usize,
    start_codes: usize,
    id_deltas: usize,
    id_range_offsets: usize,
}

impl<'font> Format4<'font> {
    /// Validates one format-4 subtable at a `cmap`-relative offset.
    pub(super) fn parse(cmap: Bytes<'font>, offset: usize) -> Result<Self, FontError> {
        if cmap.u16(offset) != Some(4) {
            return Err(FontError::InvalidCharacterMap);
        }
        let length = usize::from(cmap.u16(offset + 2).ok_or(FontError::InvalidCharacterMap)?);
        let table = cmap
            .range(offset, length)
            .ok_or(FontError::InvalidCharacterMap)?;
        let segment_words = table.u16(6).ok_or(FontError::InvalidCharacterMap)?;
        if segment_words == 0 || segment_words % 2 != 0 {
            return Err(FontError::InvalidCharacterMap);
        }
        let segment_count = usize::from(segment_words / 2);
        let segments_length = segment_count
            .checked_mul(SEGMENT_WORD_LENGTH)
            .and_then(|length| length.checked_mul(4))
            .and_then(|length| HEADER_LENGTH.checked_add(length))
            .and_then(|length| length.checked_add(2))
            .ok_or(FontError::InvalidCharacterMap)?;
        if length < segments_length {
            return Err(FontError::InvalidCharacterMap);
        }
        let end_codes = HEADER_LENGTH;
        let reserved_pad = end_codes + segment_count * SEGMENT_WORD_LENGTH;
        if table.u16(reserved_pad) != Some(0) {
            return Err(FontError::InvalidCharacterMap);
        }
        let start_codes = reserved_pad + SEGMENT_WORD_LENGTH;
        let id_deltas = start_codes + segment_count * SEGMENT_WORD_LENGTH;
        let id_range_offsets = id_deltas + segment_count * SEGMENT_WORD_LENGTH;
        let glyph_array = id_range_offsets + segment_count * SEGMENT_WORD_LENGTH;

        let mut previous_end = None;
        for index in 0..segment_count {
            let end = word(table, end_codes, index)?;
            let start = word(table, start_codes, index)?;
            if start > end || previous_end.is_some_and(|value| start <= value) {
                return Err(FontError::InvalidCharacterMap);
            }
            let range_offset = word(table, id_range_offsets, index)?;
            if range_offset != 0 {
                if range_offset % 2 != 0 {
                    return Err(FontError::InvalidCharacterMap);
                }
                let location = id_range_offsets + index * SEGMENT_WORD_LENGTH;
                let first = location
                    .checked_add(usize::from(range_offset))
                    .ok_or(FontError::InvalidCharacterMap)?;
                let span = usize::from(end - start)
                    .checked_mul(SEGMENT_WORD_LENGTH)
                    .ok_or(FontError::InvalidCharacterMap)?;
                let last = first
                    .checked_add(span)
                    .and_then(|offset| offset.checked_add(SEGMENT_WORD_LENGTH))
                    .ok_or(FontError::InvalidCharacterMap)?;
                if first < glyph_array || last > length {
                    return Err(FontError::InvalidCharacterMap);
                }
            }
            previous_end = Some(end);
        }
        if word(table, start_codes, segment_count - 1)? != u16::MAX
            || word(table, end_codes, segment_count - 1)? != u16::MAX
        {
            return Err(FontError::InvalidCharacterMap);
        }

        Ok(Self {
            table,
            segment_count,
            start_codes,
            id_deltas,
            id_range_offsets,
        })
    }

    /// Looks up one Basic Multilingual Plane scalar through validated segments.
    pub(super) fn glyph_id(&self, character: char) -> Option<GlyphId> {
        let code = u16::try_from(u32::from(character)).ok()?;
        let index = self.find_segment(code)?;
        let start = word(self.table, self.start_codes, index).ok()?;
        if code < start {
            return None;
        }
        let delta = self
            .table
            .i16(self.id_deltas + index * SEGMENT_WORD_LENGTH)?;
        let range_offset = word(self.table, self.id_range_offsets, index).ok()?;
        let raw = if range_offset == 0 {
            code
        } else {
            let location = self.id_range_offsets + index * SEGMENT_WORD_LENGTH;
            let glyph_offset = location
                .checked_add(usize::from(range_offset))?
                .checked_add(usize::from(code - start) * SEGMENT_WORD_LENGTH)?;
            word_at(self.table, glyph_offset).ok()?
        };
        if raw == 0 {
            return None;
        }
        let value = raw.wrapping_add_signed(delta);
        (value != 0).then(|| GlyphId::new(value))
    }

    fn find_segment(&self, code: u16) -> Option<usize> {
        let mut lower = 0;
        let mut upper = self.segment_count;
        while lower < upper {
            let index = lower + (upper - lower) / 2;
            let end = word(self.table, HEADER_LENGTH, index).ok()?;
            if code > end {
                lower = index + 1;
            } else {
                upper = index;
            }
        }
        (lower < self.segment_count).then_some(lower)
    }
}

fn word(table: Bytes<'_>, start: usize, index: usize) -> Result<u16, FontError> {
    let offset = start
        .checked_add(
            index
                .checked_mul(SEGMENT_WORD_LENGTH)
                .ok_or(FontError::InvalidCharacterMap)?,
        )
        .ok_or(FontError::InvalidCharacterMap)?;
    word_at(table, offset)
}

fn word_at(table: Bytes<'_>, offset: usize) -> Result<u16, FontError> {
    table.u16(offset).ok_or(FontError::InvalidCharacterMap)
}
