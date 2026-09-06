//! Validated GPOS class-definition lookup.

use crate::{FontError, GlyphId, bytes::Bytes};

/// One GPOS class definition borrowed from a validated positioning source.
pub(super) enum ClassDefinition<'font> {
    /// A contiguous glyph range with one class value per glyph.
    Values {
        start: u16,
        classes: Bytes<'font>,
        count: usize,
    },
    /// Sorted, non-overlapping glyph ranges with one shared class per range.
    Ranges { ranges: Bytes<'font>, count: usize },
}

impl<'font> ClassDefinition<'font> {
    /// Parses one supported class definition with values bounded by a matrix width.
    pub(super) fn parse(
        source: Bytes<'font>,
        glyph_count: usize,
        class_count: usize,
    ) -> Result<Self, FontError> {
        if class_count == 0 {
            return Err(FontError::InvalidFace);
        }
        match source.u16(0) {
            Some(1) => Self::values(source, glyph_count, class_count),
            Some(2) => Self::ranges(source, glyph_count, class_count),
            _ => Err(FontError::InvalidFace),
        }
    }

    /// Returns this glyph's class, using the OpenType default class zero on a miss.
    pub(super) fn class_for(&self, glyph: GlyphId) -> u16 {
        match self {
            Self::Values {
                start,
                classes,
                count,
            } => {
                let index = glyph.value().checked_sub(*start).map(usize::from);
                index
                    .filter(|index| *index < *count)
                    .and_then(|index| classes.u16(index * 2))
                    .unwrap_or(0)
            }
            Self::Ranges { ranges, count } => range_class(*ranges, *count, glyph).unwrap_or(0),
        }
    }

    fn values(
        source: Bytes<'font>,
        glyph_count: usize,
        class_count: usize,
    ) -> Result<Self, FontError> {
        let start = source.u16(2).ok_or(FontError::InvalidFace)?;
        let count = usize::from(source.u16(4).ok_or(FontError::InvalidFace)?);
        let end = usize::from(start)
            .checked_add(count)
            .ok_or(FontError::InvalidFace)?;
        if end > glyph_count {
            return Err(FontError::InvalidFace);
        }
        let classes = source
            .range(6, count.checked_mul(2).ok_or(FontError::InvalidFace)?)
            .ok_or(FontError::InvalidFace)?;
        for index in 0..count {
            if usize::from(classes.u16(index * 2).ok_or(FontError::InvalidFace)?) >= class_count {
                return Err(FontError::InvalidFace);
            }
        }
        Ok(Self::Values {
            start,
            classes,
            count,
        })
    }

    fn ranges(
        source: Bytes<'font>,
        glyph_count: usize,
        class_count: usize,
    ) -> Result<Self, FontError> {
        let count = usize::from(source.u16(2).ok_or(FontError::InvalidFace)?);
        let ranges = source
            .range(4, count.checked_mul(6).ok_or(FontError::InvalidFace)?)
            .ok_or(FontError::InvalidFace)?;
        let mut prior_end = None;
        for index in 0..count {
            let offset = index * 6;
            let start = ranges.u16(offset).ok_or(FontError::InvalidFace)?;
            let end = ranges.u16(offset + 2).ok_or(FontError::InvalidFace)?;
            let class = ranges.u16(offset + 4).ok_or(FontError::InvalidFace)?;
            if start > end
                || usize::from(end) >= glyph_count
                || usize::from(class) >= class_count
                || prior_end.is_some_and(|previous| previous >= start)
            {
                return Err(FontError::InvalidFace);
            }
            prior_end = Some(end);
        }
        Ok(Self::Ranges { ranges, count })
    }
}

/// Finds one glyph in a validated sorted class-range array.
fn range_class(ranges: Bytes<'_>, count: usize, glyph: GlyphId) -> Option<u16> {
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
            return ranges.u16(offset + 4);
        }
    }
    None
}
