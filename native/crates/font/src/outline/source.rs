//! TrueType outline-table validation and constant-time glyph location.

use crate::{
    FontError, GlyphId,
    bytes::Bytes,
    outline::{GlyphOutline, GlyphOutlineError, composite, simple},
};

const HEAD_MINIMUM_LENGTH: usize = 54;
const HEAD_MAGIC: u32 = 0x5F0F_3CF5;
const INDEX_TO_LOC_FORMAT_OFFSET: usize = 50;
const MAXP_VERSION: u32 = 0x0001_0000;
const MAXP_MINIMUM_LENGTH: usize = 32;
const GLYPH_COUNT_OFFSET: usize = 4;
const MAX_COMPOSITE_DEPTH: usize = 16;

/// One validated borrowed location index and glyph-data table.
pub(crate) struct OutlineSource<'font> {
    locations: Bytes<'font>,
    glyph_data: Bytes<'font>,
    glyph_count: usize,
    location_format: LocationFormat,
}

#[derive(Clone, Copy)]
enum LocationFormat {
    Short,
    Long,
}

impl LocationFormat {
    const fn entry_length(self) -> usize {
        match self {
            Self::Short => 2,
            Self::Long => 4,
        }
    }
}

impl<'font> OutlineSource<'font> {
    /// Parses a complete optional set of TrueType outline tables.
    pub(crate) fn optional(
        head: Option<Bytes<'font>>,
        maximum_profile: Option<Bytes<'font>>,
        locations: Option<Bytes<'font>>,
        glyph_data: Option<Bytes<'font>>,
        metric_source_present: bool,
    ) -> Result<Option<Self>, FontError> {
        if locations.is_none() && glyph_data.is_none() {
            if (head.is_none() && maximum_profile.is_none()) || metric_source_present {
                return Ok(None);
            }
            return Err(FontError::InvalidFace);
        }
        let source = Self::parse(
            head.ok_or(FontError::InvalidFace)?,
            maximum_profile.ok_or(FontError::InvalidFace)?,
            locations.ok_or(FontError::InvalidFace)?,
            glyph_data.ok_or(FontError::InvalidFace)?,
        )?;
        Ok(Some(source))
    }

    fn parse(
        head: Bytes<'font>,
        maximum_profile: Bytes<'font>,
        locations: Bytes<'font>,
        glyph_data: Bytes<'font>,
    ) -> Result<Self, FontError> {
        head.range(0, HEAD_MINIMUM_LENGTH)
            .ok_or(FontError::InvalidFace)?;
        if head.u32(12) != Some(HEAD_MAGIC) {
            return Err(FontError::InvalidFace);
        }
        let location_format = match head.i16(INDEX_TO_LOC_FORMAT_OFFSET) {
            Some(0) => LocationFormat::Short,
            Some(1) => LocationFormat::Long,
            _ => return Err(FontError::InvalidFace),
        };
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
        if glyph_count == 0 {
            return Err(FontError::InvalidFace);
        }
        let expected_length = glyph_count
            .checked_add(1)
            .and_then(|count| count.checked_mul(location_format.entry_length()))
            .ok_or(FontError::InvalidFace)?;
        if locations.len() != expected_length || !glyph_data.len().is_multiple_of(2) {
            return Err(FontError::InvalidFace);
        }

        let source = Self {
            locations,
            glyph_data,
            glyph_count,
            location_format,
        };
        source.validate_locations()?;
        Ok(source)
    }

    /// Extracts one bounded simple or translated composite glyph from its source.
    pub(crate) fn glyph_outline(&self, glyph: GlyphId) -> Result<GlyphOutline, GlyphOutlineError> {
        self.glyph_outline_at(usize::from(glyph.value()), &mut [0; MAX_COMPOSITE_DEPTH], 0)
    }

    fn glyph_outline_at(
        &self,
        index: usize,
        ancestors: &mut [u16; MAX_COMPOSITE_DEPTH],
        depth: usize,
    ) -> Result<GlyphOutline, GlyphOutlineError> {
        if index >= self.glyph_count {
            return Err(GlyphOutlineError::InvalidGlyphId);
        }
        if self.location(index)? == self.location(index + 1)? {
            return Ok(GlyphOutline::empty());
        }
        let glyph = self.glyph_bytes(index)?;
        match glyph.i16(0) {
            Some(contour_count) if contour_count >= 0 => simple::parse(glyph),
            Some(-1) => self.composite_outline(index, glyph, ancestors, depth),
            _ => Err(GlyphOutlineError::MalformedOutline),
        }
    }

    fn composite_outline(
        &self,
        index: usize,
        glyph: Bytes<'font>,
        ancestors: &mut [u16; MAX_COMPOSITE_DEPTH],
        depth: usize,
    ) -> Result<GlyphOutline, GlyphOutlineError> {
        if depth == MAX_COMPOSITE_DEPTH {
            return Err(GlyphOutlineError::ComplexityLimitExceeded);
        }
        let glyph_id = u16::try_from(index).map_err(|_| GlyphOutlineError::InvalidGlyphId)?;
        if ancestors[..depth].contains(&glyph_id) {
            return Err(GlyphOutlineError::CompositeCycle);
        }
        ancestors[depth] = glyph_id;
        let composite = composite::parse(glyph)?;
        let mut result = GlyphOutline::new(composite.bounds, Vec::new(), Vec::new());
        for component in composite.components {
            let child =
                self.glyph_outline_at(usize::from(component.glyph.value()), ancestors, depth + 1)?;
            result.append_translated(child, component.x_offset, component.y_offset)?;
        }
        Ok(result)
    }

    fn glyph_bytes(&self, index: usize) -> Result<Bytes<'font>, GlyphOutlineError> {
        let start = self.location(index)?;
        let end = self.location(index + 1)?;
        let length = end
            .checked_sub(start)
            .ok_or(GlyphOutlineError::MalformedOutline)?;
        self.glyph_data
            .range(start, length)
            .ok_or(GlyphOutlineError::MalformedOutline)
    }

    fn validate_locations(&self) -> Result<(), FontError> {
        let mut previous = None;
        for index in 0..=self.glyph_count {
            let location = self.location_for_face(index)?;
            if (index == 0 && location != 0)
                || location % 2 != 0
                || previous.is_some_and(|last| location < last)
            {
                return Err(FontError::InvalidFace);
            }
            previous = Some(location);
        }
        if previous != Some(self.glyph_data.len()) {
            return Err(FontError::InvalidFace);
        }
        Ok(())
    }

    fn location(&self, index: usize) -> Result<usize, GlyphOutlineError> {
        self.location_for_face(index)
            .map_err(|_| GlyphOutlineError::MalformedOutline)
    }

    fn location_for_face(&self, index: usize) -> Result<usize, FontError> {
        let offset = index
            .checked_mul(self.location_format.entry_length())
            .ok_or(FontError::InvalidFace)?;
        match self.location_format {
            LocationFormat::Short => self
                .locations
                .u16(offset)
                .map(|value| usize::from(value) * 2)
                .ok_or(FontError::InvalidFace),
            LocationFormat::Long => {
                usize::try_from(self.locations.u32(offset).ok_or(FontError::InvalidFace)?)
                    .map_err(|_| FontError::InvalidFace)
            }
        }
    }
}
