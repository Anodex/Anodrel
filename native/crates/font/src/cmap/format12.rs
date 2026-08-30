//! Validated format-12 Unicode group maps.

use crate::{FontError, GlyphId, bytes::Bytes};

const HEADER_LENGTH: usize = 16;
const GROUP_LENGTH: usize = 12;
const MAX_UNICODE: u32 = 0x10_FFFF;

/// One validated format-12 map borrowed from the face bytes.
pub(crate) struct Format12<'font> {
    table: Bytes<'font>,
    group_count: usize,
}

impl<'font> Format12<'font> {
    /// Validates one format-12 subtable at a `cmap`-relative offset.
    pub(super) fn parse(cmap: Bytes<'font>, offset: usize) -> Result<Self, FontError> {
        if cmap.u16(offset) != Some(12) || cmap.u16(offset + 2) != Some(0) {
            return Err(FontError::InvalidCharacterMap);
        }
        let length = usize::try_from(cmap.u32(offset + 4).ok_or(FontError::InvalidCharacterMap)?)
            .map_err(|_| FontError::InvalidCharacterMap)?;
        let table = cmap
            .range(offset, length)
            .ok_or(FontError::InvalidCharacterMap)?;
        let group_count = usize::try_from(table.u32(12).ok_or(FontError::InvalidCharacterMap)?)
            .map_err(|_| FontError::InvalidCharacterMap)?;
        let groups_length = group_count
            .checked_mul(GROUP_LENGTH)
            .and_then(|length| HEADER_LENGTH.checked_add(length))
            .ok_or(FontError::InvalidCharacterMap)?;
        if length < groups_length {
            return Err(FontError::InvalidCharacterMap);
        }

        let mut previous_end = None;
        for index in 0..group_count {
            let (start, end, start_glyph) = read_group(table, index)?;
            if start > end || end > MAX_UNICODE || previous_end.is_some_and(|value| start <= value)
            {
                return Err(FontError::InvalidCharacterMap);
            }
            let span = end - start;
            if start_glyph
                .checked_add(span)
                .is_none_or(|last| last > u32::from(u16::MAX))
            {
                return Err(FontError::InvalidCharacterMap);
            }
            previous_end = Some(end);
        }
        Ok(Self { table, group_count })
    }

    /// Looks up one scalar with a binary search over validated groups.
    pub(super) fn glyph_id(&self, character: char) -> Option<GlyphId> {
        let code = u32::from(character);
        let mut lower = 0;
        let mut upper = self.group_count;
        while lower < upper {
            let index = lower + (upper - lower) / 2;
            let (start, end, start_glyph) = read_group(self.table, index).ok()?;
            if code < start {
                upper = index;
            } else if code > end {
                lower = index + 1;
            } else {
                let value = start_glyph.checked_add(code - start)?;
                let value = u16::try_from(value).ok()?;
                return (value != 0).then(|| GlyphId::new(value));
            }
        }
        None
    }
}

fn read_group(table: Bytes<'_>, index: usize) -> Result<(u32, u32, u32), FontError> {
    let offset = HEADER_LENGTH
        .checked_add(
            index
                .checked_mul(GROUP_LENGTH)
                .ok_or(FontError::InvalidCharacterMap)?,
        )
        .ok_or(FontError::InvalidCharacterMap)?;
    Ok((
        table.u32(offset).ok_or(FontError::InvalidCharacterMap)?,
        table
            .u32(offset + 4)
            .ok_or(FontError::InvalidCharacterMap)?,
        table
            .u32(offset + 8)
            .ok_or(FontError::InvalidCharacterMap)?,
    ))
}
