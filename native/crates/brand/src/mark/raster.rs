//! Embedded authored mark asset and its bounded scaled-image cache.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use anodrel_canvas::Image;

/// The authored mark, as straight-alpha `B, G, R, A` at [`RASTER_SIDE`] square.
///
/// This is the brand asset itself, not a reconstruction of it. It is stored
/// pre-decoded so the platform ships no image decoder and takes no dependency
/// to display its own logo. See `assets/README.md` for provenance and for the
/// step that regenerates it.
static MARK_BYTES: &[u8] = include_bytes!("../../assets/mark-512.bgra");

/// Edge length of the embedded asset.
pub const RASTER_SIDE: u32 = 512;

/// Smallest edge at which the raster is preferred.
///
/// Below this the asset has to be reduced so far that its chamfers collapse
/// into a smear, while the geometry in this module stays crisp because it is
/// rasterized at the size actually asked for.
pub const RASTER_MIN_EDGE: f32 = 64.0;

/// Granularity of the scaled-raster cache, in pixels.
///
/// A reveal changes the mark's size slightly on every frame. Rounding up to a
/// bucket means the expensive filtered reduction happens once, and the small
/// remaining difference is absorbed by the bilinear sample at draw time.
const RASTER_BUCKET: u32 = 64;

/// Returns the authored mark at its stored resolution.
///
/// Returns `None` only if the embedded asset does not match [`RASTER_SIDE`],
/// which would mean the asset and this constant disagree.
#[must_use]
pub fn raster() -> Option<&'static Image> {
    static DECODED: OnceLock<Option<Image>> = OnceLock::new();
    DECODED
        .get_or_init(|| Image::from_bgra_bytes(RASTER_SIDE, RASTER_SIDE, MARK_BYTES))
        .as_ref()
}

thread_local! {
    /// Filtered reductions of the mark, keyed by bucket edge.
    static SCALED: RefCell<Vec<(u32, Rc<Image>)>> = const { RefCell::new(Vec::new()) };
}

/// Returns the mark reduced to the bucket covering `edge`, caching the result.
pub(super) fn scaled_raster(edge: f32) -> Option<Rc<Image>> {
    let source = raster()?;
    let wanted = (edge.ceil().max(1.0) as u32).min(RASTER_SIDE);
    let bucket = wanted.div_ceil(RASTER_BUCKET) * RASTER_BUCKET;
    let bucket = bucket.clamp(RASTER_BUCKET, RASTER_SIDE);
    SCALED.with(|cache| {
        if let Some((_, image)) = cache.borrow().iter().find(|(size, _)| *size == bucket) {
            return Some(image.clone());
        }
        let scaled = Rc::new(source.resized(bucket, bucket));
        let mut entries = cache.borrow_mut();
        // A handful of sizes covers every surface; past that, start over rather
        // than retain reductions no longer being asked for.
        if entries.len() >= 6 {
            entries.clear();
        }
        entries.push((bucket, scaled.clone()));
        Some(scaled)
    })
}
