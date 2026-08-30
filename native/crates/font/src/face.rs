//! TrueType face-directory validation and public face values.

use crate::{
    FontError,
    bytes::Bytes,
    cmap::{CharacterMap, parse_character_map},
};

const TRUETYPE_SFNT_VERSION: u32 = 0x0001_0000;
const SFNT_HEADER_LENGTH: usize = 12;
const TABLE_RECORD_LENGTH: usize = 16;
const MAX_TABLES: usize = 64;
const CMAP_TAG: u32 = u32::from_be_bytes(*b"cmap");

/// One nonzero glyph identifier in a parsed face.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GlyphId(u16);

impl GlyphId {
    /// Returns the font-local numeric glyph identifier.
    pub const fn value(self) -> u16 {
        self.0
    }

    pub(crate) const fn new(value: u16) -> Self {
        Self(value)
    }
}

/// A validated borrowed TrueType face with one selected Unicode character map.
pub struct FontFace<'font> {
    character_map: CharacterMap<'font>,
}

impl<'font> FontFace<'font> {
    /// Parses one bounded caller-owned TrueType face without copying its bytes.
    pub fn parse(source: &'font [u8]) -> Result<Self, FontError> {
        let bytes = Bytes::new(source);
        let version = bytes.u32(0).ok_or(FontError::InvalidFace)?;
        if version != TRUETYPE_SFNT_VERSION {
            return Err(FontError::UnsupportedFace);
        }
        let table_count = usize::from(bytes.u16(4).ok_or(FontError::InvalidFace)?);
        if table_count > MAX_TABLES {
            return Err(FontError::InvalidFace);
        }
        let directory_length = table_count
            .checked_mul(TABLE_RECORD_LENGTH)
            .and_then(|length| SFNT_HEADER_LENGTH.checked_add(length))
            .ok_or(FontError::InvalidFace)?;
        bytes
            .range(0, directory_length)
            .ok_or(FontError::InvalidFace)?;

        let mut character_map = None;
        for index in 0..table_count {
            let record = SFNT_HEADER_LENGTH + index * TABLE_RECORD_LENGTH;
            let tag = bytes.u32(record).ok_or(FontError::InvalidFace)?;
            let offset = usize::try_from(bytes.u32(record + 8).ok_or(FontError::InvalidFace)?)
                .map_err(|_| FontError::InvalidFace)?;
            let length = usize::try_from(bytes.u32(record + 12).ok_or(FontError::InvalidFace)?)
                .map_err(|_| FontError::InvalidFace)?;
            let table = bytes.range(offset, length).ok_or(FontError::InvalidFace)?;
            if tag == CMAP_TAG && character_map.replace(table).is_some() {
                return Err(FontError::InvalidFace);
            }
        }

        let character_map =
            parse_character_map(character_map.ok_or(FontError::MissingCharacterMap)?)?;
        Ok(Self { character_map })
    }

    /// Resolves one Unicode scalar to a nonzero glyph identifier, if the face has one.
    pub fn glyph_id(&self, character: char) -> Option<GlyphId> {
        self.character_map.glyph_id(character)
    }
}

impl std::fmt::Debug for FontFace<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FontFace(..)")
    }
}
