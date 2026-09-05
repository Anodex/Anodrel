//! Face-local retention of bounded glyph coverage masks.

use std::{collections::BTreeMap, rc::Rc};

use anodrel_canvas::{Mask, Point, point};
use anodrel_font::{FontFace, GlyphId, GlyphOutlineError};

use crate::{GlyphPlacement, GlyphRenderError, coverage_mask};

/// The greatest number of masks one cache can retain.
pub const MAX_CACHED_GLYPH_MASKS: usize = 64;
/// The greatest total coverage area one cache can retain.
pub const MAX_CACHED_GLYPH_PIXELS: usize = 2_097_152;
const MAX_BASELINE_ABS: f32 = 1_048_576.0;

/// A closed failure from a face-local glyph-mask lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphCacheError {
    /// The requested baseline is non-finite or outside the fixed canvas range.
    InvalidBaseline,
    /// The selected face could not produce a bounded outline for this glyph.
    Outline(GlyphOutlineError),
    /// The glyph could not satisfy the existing bounded rendering contract.
    Render(GlyphRenderError),
}

/// One retained local mask plus the whole-pixel translation for one request.
#[derive(Clone)]
pub struct CachedGlyphMask {
    mask: Rc<Mask>,
    offset_x: i32,
    offset_y: i32,
}

impl CachedGlyphMask {
    /// Returns the coverage mask produced at this request's fractional phase.
    pub fn mask(&self) -> &Mask {
        &self.mask
    }

    /// Returns the whole-pixel horizontal translation for this requested baseline.
    pub const fn offset_x(&self) -> i32 {
        self.offset_x
    }

    /// Returns the whole-pixel vertical translation for this requested baseline.
    pub const fn offset_y(&self) -> i32 {
        self.offset_y
    }
}

/// A bounded least-recently-used glyph coverage cache for one validated face.
pub struct GlyphMaskCache<'font> {
    face: &'font FontFace<'font>,
    entries: BTreeMap<CacheKey, CacheEntry>,
    retained_pixels: usize,
    access_clock: u64,
}

impl<'font> GlyphMaskCache<'font> {
    /// Builds an empty cache that borrows exactly one already-validated face.
    pub fn new(face: &'font FontFace<'font>) -> Self {
        Self {
            face,
            entries: BTreeMap::new(),
            retained_pixels: 0,
            access_clock: 0,
        }
    }

    /// Returns one cached or newly rasterized glyph mask at a requested baseline.
    ///
    /// Equal glyph, scale, and fractional-baseline values reuse coverage. The
    /// returned offsets carry only the baseline's whole-pixel translation.
    pub fn mask_at(
        &mut self,
        glyph: GlyphId,
        baseline: Point,
        pixels_per_design_unit: f32,
    ) -> Result<CachedGlyphMask, GlyphCacheError> {
        let (offset_x, phase_x) = split_coordinate(baseline.x)?;
        let (offset_y, phase_y) = split_coordinate(baseline.y)?;
        let placement = GlyphPlacement::new(point(phase_x, phase_y), pixels_per_design_unit)
            .map_err(GlyphCacheError::Render)?;
        let key = CacheKey {
            glyph,
            scale_bits: pixels_per_design_unit.to_bits(),
            phase_x_bits: phase_x.to_bits(),
            phase_y_bits: phase_y.to_bits(),
        };

        if let Some(mask) = self.cached_mask(key) {
            return Ok(CachedGlyphMask {
                mask,
                offset_x,
                offset_y,
            });
        }

        let outline = self
            .face
            .glyph_outline(glyph)
            .map_err(GlyphCacheError::Outline)?;
        let mask = Rc::new(
            coverage_mask(&outline.quadratic_path(), placement).map_err(GlyphCacheError::Render)?,
        );
        self.retain(key, Rc::clone(&mask));
        Ok(CachedGlyphMask {
            mask,
            offset_x,
            offset_y,
        })
    }

    /// Returns the number of coverage masks retained by this cache.
    pub fn retained_mask_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the total retained coverage area in pixels.
    pub const fn retained_pixel_count(&self) -> usize {
        self.retained_pixels
    }

    fn cached_mask(&mut self, key: CacheKey) -> Option<Rc<Mask>> {
        let access = self.next_access();
        let entry = self.entries.get_mut(&key)?;
        entry.last_access = access;
        Some(Rc::clone(&entry.mask))
    }

    fn retain(&mut self, key: CacheKey, mask: Rc<Mask>) {
        let pixels = mask_pixel_count(&mask);
        if pixels > MAX_CACHED_GLYPH_PIXELS {
            return;
        }
        while !self.entries.is_empty()
            && (self.entries.len() >= MAX_CACHED_GLYPH_MASKS
                || self.retained_pixels.saturating_add(pixels) > MAX_CACHED_GLYPH_PIXELS)
        {
            self.evict_oldest();
        }
        self.retained_pixels += pixels;
        let last_access = self.next_access();
        self.entries.insert(
            key,
            CacheEntry {
                mask,
                pixels,
                last_access,
            },
        );
    }

    fn evict_oldest(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(key, _)| *key)
            .expect("a non-empty cache has an oldest entry");
        let entry = self
            .entries
            .remove(&oldest)
            .expect("the selected cache entry still exists");
        self.retained_pixels -= entry.pixels;
    }

    fn next_access(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            let mut keys: Vec<_> = self.entries.keys().copied().collect();
            keys.sort_by_key(|key| self.entries[key].last_access);
            for (index, key) in keys.into_iter().enumerate() {
                self.entries
                    .get_mut(&key)
                    .expect("cache key came from these entries")
                    .last_access = u64::try_from(index + 1).expect("cache length fits u64");
            }
            self.access_clock = u64::try_from(self.entries.len()).expect("cache length fits u64");
        }
        self.access_clock += 1;
        self.access_clock
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CacheKey {
    glyph: GlyphId,
    scale_bits: u32,
    phase_x_bits: u32,
    phase_y_bits: u32,
}

struct CacheEntry {
    mask: Rc<Mask>,
    pixels: usize,
    last_access: u64,
}

fn split_coordinate(value: f32) -> Result<(i32, f32), GlyphCacheError> {
    if !value.is_finite() || value.abs() > MAX_BASELINE_ABS {
        return Err(GlyphCacheError::InvalidBaseline);
    }
    let whole = value.floor();
    let phase = value - whole;
    Ok((whole as i32, if phase == 0.0 { 0.0 } else { phase }))
}

fn mask_pixel_count(mask: &Mask) -> usize {
    usize::try_from(u64::from(mask.width()) * u64::from(mask.height()))
        .expect("bounded glyph mask area fits usize")
}
