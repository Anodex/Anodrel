//! Bounded horizontal line and glyph metrics from validated TrueType tables.

use crate::{FontError, GlyphId, bytes::Bytes};

const HEAD_MINIMUM_LENGTH: usize = 54;
const HEAD_MAGIC_OFFSET: usize = 12;
const HEAD_MAGIC: u32 = 0x5F0F_3CF5;
const UNITS_PER_EM_OFFSET: usize = 18;
const MIN_UNITS_PER_EM: u16 = 16;
const MAX_UNITS_PER_EM: u16 = 16_384;
const MAXP_VERSION: u32 = 0x0001_0000;
const MAXP_MINIMUM_LENGTH: usize = 32;
const GLYPH_COUNT_OFFSET: usize = 4;
const HHEA_VERSION: u32 = 0x0001_0000;
const HHEA_LENGTH: usize = 36;
const ASCENDER_OFFSET: usize = 4;
const DESCENDER_OFFSET: usize = 6;
const LINE_GAP_OFFSET: usize = 8;
const RESERVED_OFFSETS: [usize; 4] = [24, 26, 28, 30];
const METRIC_DATA_FORMAT_OFFSET: usize = 32;
const HORIZONTAL_METRIC_COUNT_OFFSET: usize = 34;
const LONG_METRIC_LENGTH: usize = 4;
const SIDE_BEARING_LENGTH: usize = 2;

/// Closed outcomes from a horizontal-metric request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontMetricError {
    /// The parsed face contains no complete validated horizontal-metric source.
    MetricsUnavailable,
    /// The supplied glyph identifier is outside the metric source's glyph range.
    InvalidGlyphId,
}

/// One font-wide horizontal line metric set in font design units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontMetrics {
    units_per_em: u16,
    ascender: i16,
    descender: i16,
    line_gap: i16,
}

impl FontMetrics {
    /// Returns the font design units contained in one em.
    pub const fn units_per_em(self) -> u16 {
        self.units_per_em
    }

    /// Returns the typographic ascent in font design units.
    pub const fn ascender(self) -> i16 {
        self.ascender
    }

    /// Returns the typographic descent in font design units.
    pub const fn descender(self) -> i16 {
        self.descender
    }

    /// Returns the typographic line gap in font design units.
    pub const fn line_gap(self) -> i16 {
        self.line_gap
    }
}

/// One glyph's horizontal advance and left side bearing in font design units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HorizontalMetric {
    advance_width: u16,
    left_side_bearing: i16,
}

impl HorizontalMetric {
    /// Returns the pen advance after this glyph in font design units.
    pub const fn advance_width(self) -> u16 {
        self.advance_width
    }

    /// Returns the glyph's left side bearing in font design units.
    pub const fn left_side_bearing(self) -> i16 {
        self.left_side_bearing
    }
}

/// One parsed borrowed source for constant-time horizontal metric reads.
pub(crate) struct MetricSource<'font> {
    horizontal_metrics: Bytes<'font>,
    glyph_count: usize,
    long_metric_count: usize,
    metrics: FontMetrics,
}

impl<'font> MetricSource<'font> {
    /// Parses the complete optional horizontal-metric table set.
    pub(crate) fn optional(
        head: Option<Bytes<'font>>,
        maximum_profile: Option<Bytes<'font>>,
        horizontal_header: Option<Bytes<'font>>,
        horizontal_metrics: Option<Bytes<'font>>,
        outline_tables_present: bool,
    ) -> Result<Option<Self>, FontError> {
        if horizontal_header.is_none() && horizontal_metrics.is_none() {
            if (head.is_none() && maximum_profile.is_none()) || outline_tables_present {
                return Ok(None);
            }
            return Err(FontError::InvalidFace);
        }
        Ok(Some(Self::parse(
            head.ok_or(FontError::InvalidFace)?,
            maximum_profile.ok_or(FontError::InvalidFace)?,
            horizontal_header.ok_or(FontError::InvalidFace)?,
            horizontal_metrics.ok_or(FontError::InvalidFace)?,
        )?))
    }

    /// Returns the face-wide values parsed with this source.
    pub(crate) const fn metrics(&self) -> FontMetrics {
        self.metrics
    }

    /// Returns the validated number of glyph IDs available to related tables.
    pub(crate) const fn glyph_count(&self) -> usize {
        self.glyph_count
    }

    /// Reads one glyph's advance and side bearing without allocating.
    pub(crate) fn horizontal_metric(
        &self,
        glyph: GlyphId,
    ) -> Result<HorizontalMetric, FontMetricError> {
        let glyph_index = usize::from(glyph.value());
        if glyph_index >= self.glyph_count {
            return Err(FontMetricError::InvalidGlyphId);
        }
        let long_metric_index = glyph_index.min(self.long_metric_count - 1);
        let advance_offset = long_metric_index * LONG_METRIC_LENGTH;
        let side_bearing_offset = if glyph_index < self.long_metric_count {
            advance_offset + SIDE_BEARING_LENGTH
        } else {
            self.long_metric_count * LONG_METRIC_LENGTH
                + (glyph_index - self.long_metric_count) * SIDE_BEARING_LENGTH
        };
        Ok(HorizontalMetric {
            advance_width: self
                .horizontal_metrics
                .u16(advance_offset)
                .ok_or(FontMetricError::InvalidGlyphId)?,
            left_side_bearing: self
                .horizontal_metrics
                .i16(side_bearing_offset)
                .ok_or(FontMetricError::InvalidGlyphId)?,
        })
    }

    fn parse(
        head: Bytes<'font>,
        maximum_profile: Bytes<'font>,
        horizontal_header: Bytes<'font>,
        horizontal_metrics: Bytes<'font>,
    ) -> Result<Self, FontError> {
        head.range(0, HEAD_MINIMUM_LENGTH)
            .ok_or(FontError::InvalidFace)?;
        if head.u32(HEAD_MAGIC_OFFSET) != Some(HEAD_MAGIC) {
            return Err(FontError::InvalidFace);
        }
        let units_per_em = head
            .u16(UNITS_PER_EM_OFFSET)
            .filter(|units| (MIN_UNITS_PER_EM..=MAX_UNITS_PER_EM).contains(units))
            .ok_or(FontError::InvalidFace)?;
        maximum_profile
            .range(0, MAXP_MINIMUM_LENGTH)
            .ok_or(FontError::InvalidFace)?;
        if maximum_profile.u32(0) != Some(MAXP_VERSION) {
            return Err(FontError::InvalidFace);
        }
        let glyph_count = usize::from(
            maximum_profile
                .u16(GLYPH_COUNT_OFFSET)
                .ok_or(FontError::InvalidFace)?,
        );
        if glyph_count == 0 || horizontal_header.len() != HHEA_LENGTH {
            return Err(FontError::InvalidFace);
        }
        if horizontal_header.u32(0) != Some(HHEA_VERSION)
            || horizontal_header.i16(METRIC_DATA_FORMAT_OFFSET) != Some(0)
            || RESERVED_OFFSETS
                .iter()
                .any(|offset| horizontal_header.i16(*offset) != Some(0))
        {
            return Err(FontError::InvalidFace);
        }
        let long_metric_count = usize::from(
            horizontal_header
                .u16(HORIZONTAL_METRIC_COUNT_OFFSET)
                .ok_or(FontError::InvalidFace)?,
        );
        if long_metric_count == 0 || long_metric_count > glyph_count {
            return Err(FontError::InvalidFace);
        }
        let expected_length = long_metric_count
            .checked_mul(LONG_METRIC_LENGTH)
            .and_then(|length| {
                glyph_count
                    .checked_sub(long_metric_count)
                    .and_then(|remaining| remaining.checked_mul(SIDE_BEARING_LENGTH))
                    .and_then(|remaining| length.checked_add(remaining))
            })
            .ok_or(FontError::InvalidFace)?;
        if horizontal_metrics.len() != expected_length {
            return Err(FontError::InvalidFace);
        }
        Ok(Self {
            horizontal_metrics,
            glyph_count,
            long_metric_count,
            metrics: FontMetrics {
                units_per_em,
                ascender: horizontal_header
                    .i16(ASCENDER_OFFSET)
                    .ok_or(FontError::InvalidFace)?,
                descender: horizontal_header
                    .i16(DESCENDER_OFFSET)
                    .ok_or(FontError::InvalidFace)?,
                line_gap: horizontal_header
                    .i16(LINE_GAP_OFFSET)
                    .ok_or(FontError::InvalidFace)?,
            },
        })
    }
}
