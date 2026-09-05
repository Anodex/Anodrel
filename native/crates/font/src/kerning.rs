//! Bounded conventional horizontal pair kerning from a borrowed `kern` table.

use crate::{FontError, GlyphId, bytes::Bytes, metrics::MetricSource};

const KERN_VERSION: u16 = 0;
const TABLE_HEADER_LENGTH: usize = 4;
const SUBTABLE_HEADER_LENGTH: usize = 6;
const FORMAT_ZERO_HEADER_LENGTH: usize = 14;
const PAIR_LENGTH: usize = 6;
const MAX_SUBTABLES: usize = 32;
const MAX_SOURCE_LENGTH: usize = TABLE_HEADER_LENGTH + MAX_SUBTABLES * u16::MAX as usize;

/// Closed outcomes from a horizontal pair-kerning lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontKerningError {
    /// The parsed face contains no complete validated horizontal-metric source.
    MetricsUnavailable,
    /// At least one supplied glyph identifier is outside the face's glyph range.
    InvalidGlyphId,
}

/// Borrowed, validated conventional pair tables for one parsed face.
pub(crate) struct KerningSource<'font> {
    glyph_count: usize,
    subtables: Vec<FormatZeroSubtable<'font>>,
}

impl<'font> KerningSource<'font> {
    /// Parses an optional `kern` source beside validated horizontal metrics.
    pub(crate) fn optional(
        source: Option<Bytes<'font>>,
        metrics: Option<&MetricSource<'font>>,
    ) -> Result<Option<Self>, FontError> {
        let Some(source) = source else {
            return Ok(None);
        };
        let metrics = metrics.ok_or(FontError::InvalidFace)?;
        if source.len() > MAX_SOURCE_LENGTH || source.u16(0) != Some(KERN_VERSION) {
            return Err(FontError::InvalidFace);
        }
        let subtable_count = usize::from(source.u16(2).ok_or(FontError::InvalidFace)?);
        if subtable_count > MAX_SUBTABLES {
            return Err(FontError::InvalidFace);
        }

        let mut cursor = TABLE_HEADER_LENGTH;
        let mut subtables = Vec::with_capacity(subtable_count);
        for _ in 0..subtable_count {
            let header = source
                .range(cursor, SUBTABLE_HEADER_LENGTH)
                .ok_or(FontError::InvalidFace)?;
            if header.u16(0) != Some(KERN_VERSION)
                || header.u8(5).is_none_or(|flags| flags & 0xF0 != 0)
            {
                return Err(FontError::InvalidFace);
            }
            let length = usize::from(header.u16(2).ok_or(FontError::InvalidFace)?);
            if length < SUBTABLE_HEADER_LENGTH {
                return Err(FontError::InvalidFace);
            }
            let subtable = source.range(cursor, length).ok_or(FontError::InvalidFace)?;
            cursor = cursor.checked_add(length).ok_or(FontError::InvalidFace)?;

            let format = header.u8(4).ok_or(FontError::InvalidFace)?;
            let flags = header.u8(5).ok_or(FontError::InvalidFace)?;
            if format == 0 && flags & 0x07 == 0x01 {
                subtables.push(FormatZeroSubtable::parse(
                    subtable,
                    metrics.glyph_count(),
                    flags & 0x08 != 0,
                )?);
            }
        }
        if !source.zero_padding_from(cursor) {
            return Err(FontError::InvalidFace);
        }
        Ok(Some(Self {
            glyph_count: metrics.glyph_count(),
            subtables,
        }))
    }

    /// Returns the signed adjustment for one ordered pair without allocating.
    pub(crate) fn horizontal_kerning(
        &self,
        left: GlyphId,
        right: GlyphId,
    ) -> Result<i32, FontKerningError> {
        if usize::from(left.value()) >= self.glyph_count
            || usize::from(right.value()) >= self.glyph_count
        {
            return Err(FontKerningError::InvalidGlyphId);
        }
        let mut adjustment = 0_i32;
        for subtable in &self.subtables {
            if let Some(value) = subtable.value_for(left, right) {
                if subtable.overrides_prior {
                    adjustment = i32::from(value);
                } else {
                    adjustment += i32::from(value);
                }
            }
        }
        Ok(adjustment)
    }
}

/// One validated, lexicographically ordered format-0 pair slice.
struct FormatZeroSubtable<'font> {
    pairs: Bytes<'font>,
    pair_count: usize,
    overrides_prior: bool,
}

impl<'font> FormatZeroSubtable<'font> {
    /// Validates the exact format-0 header and all pair records once.
    fn parse(
        source: Bytes<'font>,
        glyph_count: usize,
        overrides_prior: bool,
    ) -> Result<Self, FontError> {
        source
            .range(0, FORMAT_ZERO_HEADER_LENGTH)
            .ok_or(FontError::InvalidFace)?;
        let pair_count = usize::from(source.u16(6).ok_or(FontError::InvalidFace)?);
        let pair_bytes = pair_count
            .checked_mul(PAIR_LENGTH)
            .ok_or(FontError::InvalidFace)?;
        let expected_length = FORMAT_ZERO_HEADER_LENGTH
            .checked_add(pair_bytes)
            .ok_or(FontError::InvalidFace)?;
        if source.len() != expected_length
            || source.u16(8) != Some(search_range(pair_count))
            || source.u16(10) != Some(entry_selector(pair_count))
            || source.u16(12)
                != Some(
                    u16::try_from(pair_bytes)
                        .ok()
                        .and_then(|length| length.checked_sub(search_range(pair_count)))
                        .ok_or(FontError::InvalidFace)?,
                )
        {
            return Err(FontError::InvalidFace);
        }
        let pairs = source
            .range(FORMAT_ZERO_HEADER_LENGTH, pair_bytes)
            .ok_or(FontError::InvalidFace)?;
        let mut prior = None;
        for index in 0..pair_count {
            let pair = pair_key(pairs, index).ok_or(FontError::InvalidFace)?;
            if usize::from(pair.0) >= glyph_count
                || usize::from(pair.1) >= glyph_count
                || prior.is_some_and(|previous| previous >= pair)
            {
                return Err(FontError::InvalidFace);
            }
            prior = Some(pair);
        }
        Ok(Self {
            pairs,
            pair_count,
            overrides_prior,
        })
    }

    /// Finds one pair with binary search over the already-validated records.
    fn value_for(&self, left: GlyphId, right: GlyphId) -> Option<i16> {
        let target = (left.value(), right.value());
        let mut lower = 0;
        let mut upper = self.pair_count;
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            let key = pair_key(self.pairs, middle)?;
            if key < target {
                lower = middle + 1;
            } else if key > target {
                upper = middle;
            } else {
                return self.pairs.i16(middle * PAIR_LENGTH + 4);
            }
        }
        None
    }
}

/// Returns the validated glyph key at one pair record offset.
fn pair_key(source: Bytes<'_>, index: usize) -> Option<(u16, u16)> {
    let offset = index.checked_mul(PAIR_LENGTH)?;
    Some((source.u16(offset)?, source.u16(offset + 2)?))
}

/// Returns the format-0 binary-search range for one count.
fn search_range(pair_count: usize) -> u16 {
    let largest_power = if pair_count == 0 {
        0
    } else {
        1_usize << pair_count.ilog2()
    };
    u16::try_from(largest_power * PAIR_LENGTH).unwrap_or(0)
}

/// Returns the format-0 binary-search selector for one count.
fn entry_selector(pair_count: usize) -> u16 {
    if pair_count == 0 {
        0
    } else {
        pair_count.ilog2() as u16
    }
}
