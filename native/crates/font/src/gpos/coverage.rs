//! Validated GPOS coverage-table lookup.

use crate::{FontError, GlyphId, bytes::Bytes};

/// One GPOS coverage table borrowed from a validated positioning source.
pub(super) enum Coverage<'font> {
    /// A sorted explicit glyph array.
    Glyphs { glyphs: Bytes<'font>, count: usize },
    /// Sorted, non-overlapping glyph ranges with coverage indices.
    Ranges { ranges: Bytes<'font>, count: usize },
}

impl<'font> Coverage<'font> {
    /// Parses one complete supported coverage record from a bounded source view.
    pub(super) fn parse(source: Bytes<'font>, glyph_count: usize) -> Result<Self, FontError> {
        match source.u16(0) {
            Some(1) => Self::glyphs(source, glyph_count),
            Some(2) => Self::ranges(source, glyph_count),
            _ => Err(FontError::InvalidFace),
        }
    }

    /// Returns the selected coverage index for one glyph, if it is covered.
    pub(super) fn index_for(&self, glyph: GlyphId) -> Option<u16> {
        match self {
            Self::Glyphs { glyphs, count } => glyph_index(*glyphs, *count, glyph),
            Self::Ranges { ranges, count } => range_index(*ranges, *count, glyph),
        }
    }

    /// Returns whether every represented coverage index fits one caller bound.
    pub(super) fn indices_fit(&self, limit: usize) -> bool {
        match self {
            Self::Glyphs { count, .. } => *count <= limit,
            Self::Ranges { ranges, count } => (0..*count).all(|index| {
                let offset = index * 6;
                let start = ranges.u16(offset);
                let end = ranges.u16(offset + 2);
                let coverage = ranges.u16(offset + 4);
                match (start, end, coverage) {
                    (Some(start), Some(end), Some(coverage)) => usize::from(coverage)
                        .checked_add(usize::from(end - start) + 1)
                        .is_some_and(|end| end <= limit),
                    _ => false,
                }
            }),
        }
    }

    fn glyphs(source: Bytes<'font>, glyph_count: usize) -> Result<Self, FontError> {
        let count = usize::from(source.u16(2).ok_or(FontError::InvalidFace)?);
        let glyphs = source
            .range(4, count.checked_mul(2).ok_or(FontError::InvalidFace)?)
            .ok_or(FontError::InvalidFace)?;
        let mut prior = None;
        for index in 0..count {
            let glyph = glyphs.u16(index * 2).ok_or(FontError::InvalidFace)?;
            if usize::from(glyph) >= glyph_count || prior.is_some_and(|previous| previous >= glyph)
            {
                return Err(FontError::InvalidFace);
            }
            prior = Some(glyph);
        }
        Ok(Self::Glyphs { glyphs, count })
    }

    fn ranges(source: Bytes<'font>, glyph_count: usize) -> Result<Self, FontError> {
        let count = usize::from(source.u16(2).ok_or(FontError::InvalidFace)?);
        let ranges = source
            .range(4, count.checked_mul(6).ok_or(FontError::InvalidFace)?)
            .ok_or(FontError::InvalidFace)?;
        let mut prior_end = None;
        for index in 0..count {
            let offset = index * 6;
            let start = ranges.u16(offset).ok_or(FontError::InvalidFace)?;
            let end = ranges.u16(offset + 2).ok_or(FontError::InvalidFace)?;
            let coverage = ranges.u16(offset + 4).ok_or(FontError::InvalidFace)?;
            let length = end.checked_sub(start).ok_or(FontError::InvalidFace)?;
            if usize::from(end) >= glyph_count
                || prior_end.is_some_and(|previous| previous >= start)
                || u32::from(coverage) + u32::from(length) > u32::from(u16::MAX)
            {
                return Err(FontError::InvalidFace);
            }
            prior_end = Some(end);
        }
        Ok(Self::Ranges { ranges, count })
    }
}

/// Finds one glyph in a validated sorted coverage array.
fn glyph_index(glyphs: Bytes<'_>, count: usize, glyph: GlyphId) -> Option<u16> {
    let target = glyph.value();
    let mut lower = 0;
    let mut upper = count;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let value = glyphs.u16(middle * 2)?;
        if value < target {
            lower = middle + 1;
        } else if value > target {
            upper = middle;
        } else {
            return u16::try_from(middle).ok();
        }
    }
    None
}

/// Finds one glyph in a validated sorted coverage-range array.
fn range_index(ranges: Bytes<'_>, count: usize, glyph: GlyphId) -> Option<u16> {
    let target = glyph.value();
    let mut lower = 0;
    let mut upper = count;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let offset = middle * 6;
        let start = ranges.u16(offset)?;
        let end = ranges.u16(offset + 2)?;
        if target < start {
            upper = middle;
        } else if target > end {
            lower = middle + 1;
        } else {
            let coverage = ranges.u16(offset + 4)?;
            return coverage.checked_add(target - start);
        }
    }
    None
}
