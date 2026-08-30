//! Unicode `cmap` selection and map dispatch.

mod format12;
mod format4;

use crate::{FontError, GlyphId, bytes::Bytes};

use format4::Format4;
use format12::Format12;

const CMAP_HEADER_LENGTH: usize = 4;
const ENCODING_RECORD_LENGTH: usize = 8;
const WINDOWS_PLATFORM: u16 = 3;
const WINDOWS_UNICODE_BMP: u16 = 1;
const WINDOWS_UNICODE_FULL: u16 = 10;
const UNICODE_PLATFORM: u16 = 0;
const UNICODE_FULL: u16 = 4;

/// One selected, validated Unicode character map.
pub(crate) enum CharacterMap<'font> {
    /// A Basic Multilingual Plane segment map.
    Format4(Format4<'font>),
    /// A full-Unicode group map.
    Format12(Format12<'font>),
}

impl CharacterMap<'_> {
    /// Resolves one valid Unicode scalar through the selected map.
    pub(crate) fn glyph_id(&self, character: char) -> Option<GlyphId> {
        match self {
            Self::Format4(map) => map.glyph_id(character),
            Self::Format12(map) => map.glyph_id(character),
        }
    }
}

/// Selects and validates the highest-priority supported Unicode map.
pub(crate) fn parse_character_map(table: Bytes<'_>) -> Result<CharacterMap<'_>, FontError> {
    if table.u16(0) != Some(0) {
        return Err(FontError::InvalidCharacterMap);
    }
    let record_count = usize::from(table.u16(2).ok_or(FontError::InvalidCharacterMap)?);
    let records_length = record_count
        .checked_mul(ENCODING_RECORD_LENGTH)
        .and_then(|length| CMAP_HEADER_LENGTH.checked_add(length))
        .ok_or(FontError::InvalidCharacterMap)?;
    table
        .range(0, records_length)
        .ok_or(FontError::InvalidCharacterMap)?;

    let mut selected: Option<(u8, usize, u16)> = None;
    for index in 0..record_count {
        let record = CMAP_HEADER_LENGTH + index * ENCODING_RECORD_LENGTH;
        let platform = table.u16(record).ok_or(FontError::InvalidCharacterMap)?;
        let encoding = table
            .u16(record + 2)
            .ok_or(FontError::InvalidCharacterMap)?;
        let offset = usize::try_from(
            table
                .u32(record + 4)
                .ok_or(FontError::InvalidCharacterMap)?,
        )
        .map_err(|_| FontError::InvalidCharacterMap)?;
        let format = table.u16(offset).ok_or(FontError::InvalidCharacterMap)?;
        let candidate =
            candidate_rank(platform, encoding, format).map(|rank| (rank, offset, format));
        if candidate.is_some_and(|candidate| selected.is_none_or(|current| candidate.0 < current.0))
        {
            selected = candidate;
        }
    }

    let Some((_, offset, format)) = selected else {
        return Err(FontError::UnsupportedCharacterMap);
    };
    match format {
        4 => Format4::parse(table, offset).map(CharacterMap::Format4),
        12 => Format12::parse(table, offset).map(CharacterMap::Format12),
        _ => Err(FontError::UnsupportedCharacterMap),
    }
}

fn candidate_rank(platform: u16, encoding: u16, format: u16) -> Option<u8> {
    match (platform, encoding, format) {
        (WINDOWS_PLATFORM, WINDOWS_UNICODE_FULL, 12) => Some(0),
        (UNICODE_PLATFORM, UNICODE_FULL, 12) => Some(1),
        (WINDOWS_PLATFORM, WINDOWS_UNICODE_BMP, 4) => Some(2),
        (UNICODE_PLATFORM, 3, 4) => Some(3),
        _ => None,
    }
}
