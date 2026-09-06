//! Bounded basic-Latin GPOS pair positioning from borrowed face bytes.

mod class;
mod coverage;
mod pair;

use crate::{FontError, GlyphId, bytes::Bytes, metrics::MetricSource};

use pair::PairLookup;

const GPOS_MAJOR_VERSION: u16 = 1;
const GPOS_MINOR_VERSION: u16 = 0;
const GPOS_HEADER_LENGTH: usize = 10;
const LATIN_SCRIPT_TAG: u32 = u32::from_be_bytes(*b"latn");
const KERN_FEATURE_TAG: u32 = u32::from_be_bytes(*b"kern");
const REQUIRED_FEATURE_NONE: u16 = u16::MAX;
const MAX_SOURCE_LENGTH: usize = 2 * 1024 * 1024;
const MAX_SELECTED_PAIR_LOOKUPS: usize = 32;
const PAIR_POSITIONING_LOOKUP: u16 = 2;
const EXTENSION_POSITIONING_LOOKUP: u16 = 9;
const IGNORE_MARKS_FLAG: u16 = 0x0008;

/// Borrowed basic-Latin pair-positioning lookups selected from one GPOS table.
pub(crate) struct GposSource<'font> {
    glyph_count: usize,
    lookups: Vec<PairLookup<'font>>,
}

impl<'font> GposSource<'font> {
    /// Parses the narrow selected GPOS subset beside complete horizontal metrics.
    pub(crate) fn optional(
        source: Option<Bytes<'font>>,
        metrics: Option<&MetricSource<'font>>,
    ) -> Result<Option<Self>, FontError> {
        let Some(source) = source else {
            return Ok(None);
        };
        let metrics = metrics.ok_or(FontError::InvalidFace)?;
        source.range(0, 4).ok_or(FontError::InvalidFace)?;
        if source.len() > MAX_SOURCE_LENGTH {
            return Err(FontError::InvalidFace);
        }
        if source.u16(0) != Some(GPOS_MAJOR_VERSION) || source.u16(2) != Some(GPOS_MINOR_VERSION) {
            return Ok(None);
        }
        source
            .range(0, GPOS_HEADER_LENGTH)
            .ok_or(FontError::InvalidFace)?;
        let script_list = relative(source, 4)?;
        let feature_list = relative(source, 6)?;
        let lookup_list = relative(source, 8)?;
        let Some(language_system) = latin_default_language_system(script_list)? else {
            return Ok(None);
        };
        let Some(lookup_indices) =
            kernel_lookup_indices(language_system, feature_list, lookup_list)?
        else {
            return Ok(None);
        };
        let mut lookups = Vec::new();
        for index in lookup_indices {
            let lookup = lookup_at(lookup_list, index)?;
            let lookup_type = lookup.u16(0).ok_or(FontError::InvalidFace)?;
            let flags = lookup.u16(2).ok_or(FontError::InvalidFace)?;
            let subtable_count = usize::from(lookup.u16(4).ok_or(FontError::InvalidFace)?);
            lookup
                .range(
                    6,
                    subtable_count
                        .checked_mul(2)
                        .ok_or(FontError::InvalidFace)?,
                )
                .ok_or(FontError::InvalidFace)?;
            if !matches!(
                lookup_type,
                PAIR_POSITIONING_LOOKUP | EXTENSION_POSITIONING_LOOKUP
            ) || !matches!(flags, 0 | IGNORE_MARKS_FLAG)
            {
                continue;
            }
            for subtable_index in 0..subtable_count {
                if lookups.len() == MAX_SELECTED_PAIR_LOOKUPS {
                    return Err(FontError::InvalidFace);
                }
                let subtable = relative(lookup, 6 + subtable_index * 2)?;
                let pair_lookup = match lookup_type {
                    PAIR_POSITIONING_LOOKUP => {
                        PairLookup::optional(subtable, metrics.glyph_count())?
                    }
                    EXTENSION_POSITIONING_LOOKUP => {
                        let Some(pair_subtable) = extension_pair_subtable(subtable)? else {
                            continue;
                        };
                        PairLookup::optional(pair_subtable, metrics.glyph_count())?
                    }
                    _ => None,
                };
                if let Some(pair_lookup) = pair_lookup {
                    lookups.push(pair_lookup);
                }
            }
        }
        Ok((!lookups.is_empty()).then_some(Self {
            glyph_count: metrics.glyph_count(),
            lookups,
        }))
    }

    /// Returns the sum of selected GPOS pair adjustments for one ordered pair.
    pub(crate) fn horizontal_kerning(&self, left: GlyphId, right: GlyphId) -> i32 {
        debug_assert!(usize::from(left.value()) < self.glyph_count);
        debug_assert!(usize::from(right.value()) < self.glyph_count);
        self.lookups
            .iter()
            .filter_map(|lookup| lookup.adjustment(left, right))
            .map(i32::from)
            .sum()
    }
}

/// Selects the default language system from the exact `latn` script record.
fn latin_default_language_system(script_list: Bytes<'_>) -> Result<Option<Bytes<'_>>, FontError> {
    let count = usize::from(script_list.u16(0).ok_or(FontError::InvalidFace)?);
    script_list
        .range(2, count.checked_mul(6).ok_or(FontError::InvalidFace)?)
        .ok_or(FontError::InvalidFace)?;
    for index in 0..count {
        let offset = 2 + index * 6;
        if script_list.u32(offset) != Some(LATIN_SCRIPT_TAG) {
            continue;
        }
        let script = relative(script_list, offset + 4)?;
        script.range(0, 4).ok_or(FontError::InvalidFace)?;
        return match script.u16(0).ok_or(FontError::InvalidFace)? {
            0 => Ok(None),
            _ => relative(script, 0).map(Some),
        };
    }
    Ok(None)
}

/// Resolves selected `kern` feature lookups and validates each lookup reference.
fn kernel_lookup_indices(
    language_system: Bytes<'_>,
    feature_list: Bytes<'_>,
    lookup_list: Bytes<'_>,
) -> Result<Option<Vec<u16>>, FontError> {
    let feature_count = usize::from(feature_list.u16(0).ok_or(FontError::InvalidFace)?);
    feature_list
        .range(
            2,
            feature_count.checked_mul(6).ok_or(FontError::InvalidFace)?,
        )
        .ok_or(FontError::InvalidFace)?;
    let language_features = language_feature_indices(language_system, feature_count)?;
    let lookup_count = usize::from(lookup_list.u16(0).ok_or(FontError::InvalidFace)?);
    lookup_list
        .range(
            2,
            lookup_count.checked_mul(2).ok_or(FontError::InvalidFace)?,
        )
        .ok_or(FontError::InvalidFace)?;

    let mut indices = Vec::new();
    for feature_index in language_features {
        let record = 2 + usize::from(feature_index) * 6;
        if feature_list.u32(record) != Some(KERN_FEATURE_TAG) {
            continue;
        }
        let feature = relative(feature_list, record + 4)?;
        if feature.u16(0) != Some(0) {
            continue;
        }
        let count = usize::from(feature.u16(2).ok_or(FontError::InvalidFace)?);
        if count > MAX_SELECTED_PAIR_LOOKUPS {
            return Err(FontError::InvalidFace);
        }
        feature
            .range(4, count.checked_mul(2).ok_or(FontError::InvalidFace)?)
            .ok_or(FontError::InvalidFace)?;
        for index in 0..count {
            let lookup_index = feature.u16(4 + index * 2).ok_or(FontError::InvalidFace)?;
            if usize::from(lookup_index) >= lookup_count {
                return Err(FontError::InvalidFace);
            }
            if !indices.contains(&lookup_index) {
                indices.push(lookup_index);
            }
        }
    }
    if indices.len() > MAX_SELECTED_PAIR_LOOKUPS {
        return Err(FontError::InvalidFace);
    }
    Ok((!indices.is_empty()).then_some(indices))
}

/// Reads all feature indices from one default language system.
fn language_feature_indices(
    language_system: Bytes<'_>,
    feature_count: usize,
) -> Result<Vec<u16>, FontError> {
    language_system.range(0, 6).ok_or(FontError::InvalidFace)?;
    if language_system.u16(0) != Some(0) {
        return Err(FontError::InvalidFace);
    }
    let required = language_system.u16(2).ok_or(FontError::InvalidFace)?;
    let count = usize::from(language_system.u16(4).ok_or(FontError::InvalidFace)?);
    if count > MAX_SELECTED_PAIR_LOOKUPS {
        return Err(FontError::InvalidFace);
    }
    let selected = language_system
        .range(6, count.checked_mul(2).ok_or(FontError::InvalidFace)?)
        .ok_or(FontError::InvalidFace)?;
    let mut indices = Vec::with_capacity(count + 1);
    if required != REQUIRED_FEATURE_NONE {
        if usize::from(required) >= feature_count {
            return Err(FontError::InvalidFace);
        }
        indices.push(required);
    }
    for index in 0..count {
        let feature = selected.u16(index * 2).ok_or(FontError::InvalidFace)?;
        if usize::from(feature) >= feature_count {
            return Err(FontError::InvalidFace);
        }
        if !indices.contains(&feature) {
            indices.push(feature);
        }
    }
    Ok(indices)
}

/// Returns one selected lookup table from a validated lookup-list index.
fn lookup_at(lookup_list: Bytes<'_>, index: u16) -> Result<Bytes<'_>, FontError> {
    relative(lookup_list, 2 + usize::from(index) * 2)
}

/// Resolves one unsigned offset relative to a GPOS table or nested subtable.
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

/// Resolves one version-one extension positioning subtable to type-two pairs.
fn extension_pair_subtable(source: Bytes<'_>) -> Result<Option<Bytes<'_>>, FontError> {
    source.range(0, 8).ok_or(FontError::InvalidFace)?;
    if source.u16(0) != Some(1) || source.u16(2) != Some(PAIR_POSITIONING_LOOKUP) {
        return Ok(None);
    }
    let offset = usize::try_from(source.u32(4).ok_or(FontError::InvalidFace)?)
        .map_err(|_| FontError::InvalidFace)?;
    source
        .range(
            offset,
            source
                .len()
                .checked_sub(offset)
                .ok_or(FontError::InvalidFace)?,
        )
        .map(Some)
        .ok_or(FontError::InvalidFace)
}
