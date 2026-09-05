//! TrueType face-directory validation and public face values.

use crate::{
    FontError,
    bytes::Bytes,
    cmap::{CharacterMap, parse_character_map},
    kerning::{FontKerningError, KerningSource},
    metrics::{FontMetricError, FontMetrics, HorizontalMetric, MetricSource},
    outline::{GlyphOutline, GlyphOutlineError, OutlineSource},
};

const TRUETYPE_SFNT_VERSION: u32 = 0x0001_0000;
const SFNT_HEADER_LENGTH: usize = 12;
const TABLE_RECORD_LENGTH: usize = 16;
const MAX_TABLES: usize = 64;
const CMAP_TAG: u32 = u32::from_be_bytes(*b"cmap");
const HEAD_TAG: u32 = u32::from_be_bytes(*b"head");
const MAXP_TAG: u32 = u32::from_be_bytes(*b"maxp");
const HHEA_TAG: u32 = u32::from_be_bytes(*b"hhea");
const HMTX_TAG: u32 = u32::from_be_bytes(*b"hmtx");
const KERN_TAG: u32 = u32::from_be_bytes(*b"kern");
const LOCA_TAG: u32 = u32::from_be_bytes(*b"loca");
const GLYF_TAG: u32 = u32::from_be_bytes(*b"glyf");

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
    metric_source: Option<MetricSource<'font>>,
    kerning_source: Option<KerningSource<'font>>,
    outline_source: Option<OutlineSource<'font>>,
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
        let mut head = None;
        let mut maximum_profile = None;
        let mut horizontal_header = None;
        let mut horizontal_metrics = None;
        let mut kerning = None;
        let mut locations = None;
        let mut glyph_data = None;
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
            match tag {
                HEAD_TAG if head.replace(table).is_some() => return Err(FontError::InvalidFace),
                MAXP_TAG if maximum_profile.replace(table).is_some() => {
                    return Err(FontError::InvalidFace);
                }
                HHEA_TAG if horizontal_header.replace(table).is_some() => {
                    return Err(FontError::InvalidFace);
                }
                HMTX_TAG if horizontal_metrics.replace(table).is_some() => {
                    return Err(FontError::InvalidFace);
                }
                KERN_TAG if kerning.replace(table).is_some() => return Err(FontError::InvalidFace),
                LOCA_TAG if locations.replace(table).is_some() => {
                    return Err(FontError::InvalidFace);
                }
                GLYF_TAG if glyph_data.replace(table).is_some() => {
                    return Err(FontError::InvalidFace);
                }
                _ => {}
            }
        }

        let character_map =
            parse_character_map(character_map.ok_or(FontError::MissingCharacterMap)?)?;
        let metric_source = MetricSource::optional(
            head,
            maximum_profile,
            horizontal_header,
            horizontal_metrics,
            locations.is_some() || glyph_data.is_some(),
        )?;
        let outline_source = OutlineSource::optional(
            head,
            maximum_profile,
            locations,
            glyph_data,
            metric_source.is_some(),
        )?;
        let kerning_source = KerningSource::optional(kerning, metric_source.as_ref())?;
        Ok(Self {
            character_map,
            metric_source,
            kerning_source,
            outline_source,
        })
    }

    /// Resolves one Unicode scalar to a nonzero glyph identifier, if the face has one.
    pub fn glyph_id(&self, character: char) -> Option<GlyphId> {
        self.character_map.glyph_id(character)
    }

    /// Returns one owned bounded outline for a glyph from this face.
    pub fn glyph_outline(&self, glyph: GlyphId) -> Result<GlyphOutline, GlyphOutlineError> {
        self.outline_source
            .as_ref()
            .ok_or(GlyphOutlineError::OutlineUnavailable)?
            .glyph_outline(glyph)
    }

    /// Returns the face-wide validated horizontal metrics.
    pub fn font_metrics(&self) -> Result<FontMetrics, FontMetricError> {
        self.metric_source
            .as_ref()
            .map(MetricSource::metrics)
            .ok_or(FontMetricError::MetricsUnavailable)
    }

    /// Returns one validated glyph's horizontal advance and left side bearing.
    pub fn horizontal_metric(&self, glyph: GlyphId) -> Result<HorizontalMetric, FontMetricError> {
        self.metric_source
            .as_ref()
            .ok_or(FontMetricError::MetricsUnavailable)?
            .horizontal_metric(glyph)
    }

    /// Returns one conventional horizontal pair adjustment in font design units.
    pub fn horizontal_kerning(
        &self,
        left: GlyphId,
        right: GlyphId,
    ) -> Result<i32, FontKerningError> {
        let metrics = self
            .metric_source
            .as_ref()
            .ok_or(FontKerningError::MetricsUnavailable)?;
        if usize::from(left.value()) >= metrics.glyph_count()
            || usize::from(right.value()) >= metrics.glyph_count()
        {
            return Err(FontKerningError::InvalidGlyphId);
        }
        self.kerning_source
            .as_ref()
            .map(|source| source.horizontal_kerning(left, right))
            .unwrap_or(Ok(0))
    }
}

impl std::fmt::Debug for FontFace<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FontFace(..)")
    }
}
