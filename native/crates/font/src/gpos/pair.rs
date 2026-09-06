//! Bounded GPOS pair-positioning lookup forms.

use crate::{FontError, GlyphId, bytes::Bytes};

use super::{class::ClassDefinition, coverage::Coverage};

const FORMAT_ONE_HEADER_LENGTH: usize = 10;
const FORMAT_TWO_HEADER_LENGTH: usize = 16;
const MAX_PAIR_SETS: usize = 8_192;
const MAX_CLASS_RECORDS: usize = 65_536;
const X_ADVANCE_VALUE_FORMAT: u16 = 0x0004;

/// One borrowed pair-positioning lookup with an x-advance-only value contract.
pub(super) enum PairLookup<'font> {
    /// Explicit first/second glyph-pair adjustments.
    Pairs {
        coverage: Coverage<'font>,
        sets: Vec<PairSet<'font>>,
    },
    /// Class-derived first/second glyph-pair adjustments.
    Classes {
        coverage: Coverage<'font>,
        first_classes: ClassDefinition<'font>,
        second_classes: ClassDefinition<'font>,
        class_two_count: usize,
        values: Bytes<'font>,
    },
}

impl<'font> PairLookup<'font> {
    /// Parses one supported pair-positioning subtable or ignores an unsupported form.
    pub(super) fn optional(
        source: Bytes<'font>,
        glyph_count: usize,
    ) -> Result<Option<Self>, FontError> {
        let format = source.u16(0).ok_or(FontError::InvalidFace)?;
        let first_value_format = source.u16(4).ok_or(FontError::InvalidFace)?;
        let second_value_format = source.u16(6).ok_or(FontError::InvalidFace)?;
        if first_value_format != X_ADVANCE_VALUE_FORMAT || second_value_format != 0 {
            return Ok(None);
        }
        match format {
            1 => Self::pairs(source, glyph_count).map(Some),
            2 => Self::classes(source, glyph_count).map(Some),
            _ => Ok(None),
        }
    }

    /// Returns this lookup's horizontal adjustment for one ordered glyph pair.
    pub(super) fn adjustment(&self, left: GlyphId, right: GlyphId) -> Option<i16> {
        match self {
            Self::Pairs { coverage, sets } => coverage
                .index_for(left)
                .and_then(|index| sets.get(usize::from(index)))
                .and_then(|set| set.adjustment(right)),
            Self::Classes {
                coverage,
                first_classes,
                second_classes,
                class_two_count,
                values,
            } => coverage.index_for(left).and_then(|_| {
                let first = usize::from(first_classes.class_for(left));
                let second = usize::from(second_classes.class_for(right));
                first
                    .checked_mul(*class_two_count)
                    .and_then(|index| index.checked_add(second))
                    .and_then(|index| values.i16(index * 2))
            }),
        }
    }

    fn pairs(source: Bytes<'font>, glyph_count: usize) -> Result<Self, FontError> {
        source
            .range(0, FORMAT_ONE_HEADER_LENGTH)
            .ok_or(FontError::InvalidFace)?;
        let coverage = Coverage::parse(relative(source, 2)?, glyph_count)?;
        let set_count = usize::from(source.u16(8).ok_or(FontError::InvalidFace)?);
        if set_count > MAX_PAIR_SETS || !coverage.indices_fit(set_count) {
            return Err(FontError::InvalidFace);
        }
        source
            .range(
                FORMAT_ONE_HEADER_LENGTH,
                set_count.checked_mul(2).ok_or(FontError::InvalidFace)?,
            )
            .ok_or(FontError::InvalidFace)?;
        let mut sets = Vec::with_capacity(set_count);
        for index in 0..set_count {
            sets.push(PairSet::parse(
                relative(source, FORMAT_ONE_HEADER_LENGTH + index * 2)?,
                glyph_count,
            )?);
        }
        Ok(Self::Pairs { coverage, sets })
    }

    fn classes(source: Bytes<'font>, glyph_count: usize) -> Result<Self, FontError> {
        source
            .range(0, FORMAT_TWO_HEADER_LENGTH)
            .ok_or(FontError::InvalidFace)?;
        let class_one_count = usize::from(source.u16(12).ok_or(FontError::InvalidFace)?);
        let class_two_count = usize::from(source.u16(14).ok_or(FontError::InvalidFace)?);
        let record_count = class_one_count
            .checked_mul(class_two_count)
            .filter(|count| *count <= MAX_CLASS_RECORDS)
            .ok_or(FontError::InvalidFace)?;
        let coverage = Coverage::parse(relative(source, 2)?, glyph_count)?;
        if class_one_count == 0 || class_two_count == 0 || !coverage.indices_fit(class_one_count) {
            return Err(FontError::InvalidFace);
        }
        let first_classes =
            ClassDefinition::parse(relative(source, 8)?, glyph_count, class_one_count)?;
        let second_classes =
            ClassDefinition::parse(relative(source, 10)?, glyph_count, class_two_count)?;
        let values = source
            .range(
                FORMAT_TWO_HEADER_LENGTH,
                record_count.checked_mul(2).ok_or(FontError::InvalidFace)?,
            )
            .ok_or(FontError::InvalidFace)?;
        Ok(Self::Classes {
            coverage,
            first_classes,
            second_classes,
            class_two_count,
            values,
        })
    }
}

/// One sorted explicit second-glyph adjustment set.
pub(super) struct PairSet<'font> {
    pairs: Bytes<'font>,
    count: usize,
}

impl<'font> PairSet<'font> {
    /// Validates one x-advance-only pair set.
    fn parse(source: Bytes<'font>, glyph_count: usize) -> Result<Self, FontError> {
        let count = usize::from(source.u16(0).ok_or(FontError::InvalidFace)?);
        let pairs = source
            .range(2, count.checked_mul(4).ok_or(FontError::InvalidFace)?)
            .ok_or(FontError::InvalidFace)?;
        let mut prior = None;
        for index in 0..count {
            let second = pairs.u16(index * 4).ok_or(FontError::InvalidFace)?;
            if usize::from(second) >= glyph_count
                || prior.is_some_and(|previous| previous >= second)
            {
                return Err(FontError::InvalidFace);
            }
            prior = Some(second);
        }
        Ok(Self { pairs, count })
    }

    /// Finds one second glyph in a validated sorted pair set.
    fn adjustment(&self, right: GlyphId) -> Option<i16> {
        let target = right.value();
        let mut lower = 0;
        let mut upper = self.count;
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            let offset = middle * 4;
            let second = self.pairs.u16(offset)?;
            if second < target {
                lower = middle + 1;
            } else if second > target {
                upper = middle;
            } else {
                return self.pairs.i16(offset + 2);
            }
        }
        None
    }
}

/// Resolves one offset relative to the start of its GPOS subtable.
fn relative(source: Bytes<'_>, offset_at: usize) -> Result<Bytes<'_>, FontError> {
    let offset = usize::from(source.u16(offset_at).ok_or(FontError::InvalidFace)?);
    source
        .range(
            offset,
            source
                .len()
                .checked_sub(offset)
                .ok_or(FontError::InvalidFace)?,
        )
        .ok_or(FontError::InvalidFace)
}
