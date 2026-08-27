//! The Anodrel mark: a chevron `A` cut into four beveled pieces.
//!
//! The mark ships as the authored artwork — see [`raster`] and
//! `assets/README.md`. The geometry in this module reproduces the same shape as
//! polygons and is used below [`RASTER_MIN_EDGE`], where a reduced raster loses
//! its chamfers. It is not the source of truth for the identity; the asset is.
//!
//! Geometry is authored in a normalised unit square with `y` increasing
//! downward, then fitted to whatever size a surface needs, which is what lets
//! it share bounds with the raster exactly.
//!
//! ```text
//!            apex            The apex chevron carries the peak. Two legs
//!            /  \            descend to the baseline, and a fourth chevron
//!           /    \           forms the crossbar. Gaps between the pieces are
//!          /      \          part of the identity, not spacing to be tuned.
//!         /  /--\  \
//!        /  /    \  \
//!       /__/      \__\
//! ```

use std::cell::RefCell;

use anodrel_canvas::{Bevel, Canvas, Color, Image, Mask, Paint, Rect, Stop, point};

use crate::palette;

mod geometry;
mod raster;

use raster::scaled_raster;

pub use geometry::{Piece, depth_paint, face_paint, pieces, silhouette};
pub use raster::{RASTER_MIN_EDGE, RASTER_SIDE, raster};

/// How the mark should be rendered at a given size.
#[derive(Clone, Copy, Debug)]
pub struct MarkStyle {
    /// Chamfer width as a fraction of the mark's width.
    pub bevel_ratio: f32,
    /// Glow radius as a fraction of the mark's width.
    pub glow_ratio: f32,
    /// Number of glow composites; more deepens the core without widening it.
    pub glow_passes: u32,
    /// Opacity applied to the whole mark, for reveal animations.
    pub opacity: f32,
}

impl MarkStyle {
    /// The full hero treatment: chamfered facets and a wide bloom.
    ///
    /// Two glow passes rather than three: the mask is the most expensive thing
    /// on the surface, and past two composites the extra depth is not worth a
    /// third pass over every masked pixel on an animated frame.
    #[must_use]
    pub const fn hero() -> Self {
        Self {
            bevel_ratio: 0.015,
            glow_ratio: 0.22,
            glow_passes: 2,
            opacity: 1.0,
        }
    }

    /// A compact treatment for wordmark-scale and icon-scale drawing.
    ///
    /// The chamfer is proportionally wider because at small sizes a hairline
    /// bevel disappears into a single pixel of coverage.
    #[must_use]
    pub const fn compact() -> Self {
        Self {
            bevel_ratio: 0.026,
            glow_ratio: 0.0,
            glow_passes: 0,
            opacity: 1.0,
        }
    }

    /// Returns a copy at a new opacity.
    #[must_use]
    pub fn with_opacity(self, opacity: f32) -> Self {
        Self {
            opacity: opacity.clamp(0.0, 1.0),
            ..self
        }
    }
}

/// Draws the mark into `bounds`.
///
/// The glow is laid down first so the mark sits on top of its own light rather
/// than being washed out by it.
///
/// The body is the authored raster wherever it is large enough to hold up
/// ([`RASTER_MIN_EDGE`]); below that it is drawn from the geometry in this
/// module. Both occupy exactly the same bounds, because the asset is cropped
/// square to its artwork and the geometry fills the unit square, so nothing
/// shifts when the renderer switches between them.
pub fn draw(canvas: &mut Canvas, bounds: Rect, style: MarkStyle) {
    if bounds.is_empty() || style.opacity <= 0.0 {
        return;
    }
    let width = bounds.width();
    let edge = width.max(bounds.height());
    let scaled = (edge >= RASTER_MIN_EDGE)
        .then(|| scaled_raster(edge))
        .flatten();

    if style.glow_ratio > 0.0 && style.glow_passes > 0 {
        let glow = glow_paint(bounds).scale_alpha(style.opacity);
        let radius = width * style.glow_ratio;
        match scaled.as_deref() {
            // Take the glow's shape from the artwork's own alpha, so the light
            // matches the mark being lit rather than an approximation of it.
            Some(image) => draw_raster_glow(canvas, image, bounds, radius, style, &glow),
            None => canvas.draw_glow(&silhouette(bounds), radius, style.glow_passes, &glow),
        }
    }

    match scaled.as_deref() {
        Some(image) => canvas.draw_image(image, bounds, style.opacity),
        None => draw_geometry(canvas, bounds, style),
    }
}

/// Draws only the mark's glow, for compositing as its own layer.
pub fn draw_glow_layer(canvas: &mut Canvas, bounds: Rect, style: MarkStyle) {
    if bounds.is_empty() || style.glow_ratio <= 0.0 || style.glow_passes == 0 {
        return;
    }
    let glow = glow_paint(bounds).scale_alpha(style.opacity);
    let radius = bounds.width() * style.glow_ratio;
    let edge = bounds.width().max(bounds.height());
    match (edge >= RASTER_MIN_EDGE)
        .then(|| scaled_raster(edge))
        .flatten()
    {
        Some(image) => draw_raster_glow(canvas, &image, bounds, radius, style, &glow),
        None => canvas.draw_glow(&silhouette(bounds), radius, style.glow_passes, &glow),
    }
}

/// Draws only the mark's body, for compositing as its own layer.
pub fn draw_body_layer(canvas: &mut Canvas, bounds: Rect, style: MarkStyle) {
    if bounds.is_empty() || style.opacity <= 0.0 {
        return;
    }
    let edge = bounds.width().max(bounds.height());
    match (edge >= RASTER_MIN_EDGE)
        .then(|| scaled_raster(edge))
        .flatten()
    {
        Some(image) => canvas.draw_image(&image, bounds, style.opacity),
        None => draw_geometry(canvas, bounds, style),
    }
}

/// Returns the mark's own alpha as a coverage mask placed at `bounds`.
///
/// Effects that must stay inside the mark — a travelling highlight, say — use
/// this so they follow the real artwork rather than an outline of it.
#[must_use]
pub fn coverage_mask(bounds: Rect) -> Option<Mask> {
    if bounds.is_empty() {
        return None;
    }
    let edge = bounds.width().max(bounds.height());
    let image = (edge >= RASTER_MIN_EDGE)
        .then(|| scaled_raster(edge))
        .flatten()?;
    let origin_x = bounds.left.floor() as i32;
    let origin_y = bounds.top.floor() as i32;
    let width = bounds.width().ceil().max(1.0) as u32;
    let height = bounds.height().ceil().max(1.0) as u32;
    let mut coverage = Vec::with_capacity((width as usize) * (height as usize));
    for y in 0..height {
        let v = ((origin_y + y as i32) as f32 + 0.5 - bounds.top) / bounds.height();
        for x in 0..width {
            let u = ((origin_x + x as i32) as f32 + 0.5 - bounds.left) / bounds.width();
            coverage.push(if (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v) {
                f32::from(image.sample_bilinear(u, v).alpha) / 255.0
            } else {
                0.0
            });
        }
    }
    Mask::from_coverage(origin_x, origin_y, width, height, coverage)
}

fn glow_paint(bounds: Rect) -> Paint {
    Paint::linear(
        point(bounds.left, 0.0),
        point(bounds.right, 0.0),
        vec![
            Stop::new(0.0, palette::VIOLET.with_alpha(140)),
            Stop::new(0.5, palette::INDIGO.with_alpha(120)),
            Stop::new(1.0, palette::BLUE.with_alpha(140)),
        ],
    )
}

/// Returns a travelling highlight band for the mark, at `progress` through one pass.
///
/// The axis is tilted rather than horizontal so the band crosses the chamfers
/// at an angle, which is what makes a flat gradient read as light moving over a
/// solid rather than as a bar sliding across a picture.
///
/// `progress` runs `0.0..=1.0`; the band starts off one edge and finishes off
/// the other, so a pass has no visible entry or exit.
#[must_use]
pub fn sweep_paint(bounds: Rect, progress: f32, strength: f32) -> Paint {
    let travel = -0.25 + progress.clamp(0.0, 1.0) * 1.5;
    // Narrow enough to read as a glint catching an edge rather than as a wash
    // moving over the whole mark.
    let half_width = 0.10;
    let peak = Color::WHITE.with_alpha((strength.clamp(0.0, 1.0) * 255.0) as u8);
    let stops = vec![
        Stop::new(0.0, Color::TRANSPARENT),
        Stop::new((travel - half_width).clamp(0.0, 1.0), Color::TRANSPARENT),
        Stop::new(travel.clamp(0.0, 1.0), peak),
        Stop::new((travel + half_width).clamp(0.0, 1.0), Color::TRANSPARENT),
        Stop::new(1.0, Color::TRANSPARENT),
    ];
    // A shallow downward tilt across the mark's own box.
    let lift = bounds.height() * 0.55;
    Paint::linear(
        point(bounds.left, bounds.top - lift * 0.5),
        point(bounds.right, bounds.bottom - lift * 0.5),
        stops,
    )
}

/// Draws the mark from this module's geometry rather than from the asset.
///
/// Each piece is filled with the shared horizontal ramp, shaded vertically, and
/// chamfered — in that order, so the chamfers read as edges of a solid rather
/// than as outlines.
fn draw_geometry(canvas: &mut Canvas, bounds: Rect, style: MarkStyle) {
    let face = face_paint(bounds).scale_alpha(style.opacity);
    let depth = depth_paint(bounds).scale_alpha(style.opacity);
    let bevel = Bevel::top_left(bounds.width() * style.bevel_ratio).scaled(style.opacity);
    for (_, path) in pieces(bounds) {
        canvas.fill_beveled(&path, &face, &bevel);
        canvas.fill_path(&path, &depth);
    }
}

/// Everything about a glow mask except where it is placed on the canvas.
///
/// Two requests with equal shapes produce identical coverage, so this is the
/// cache key. `blur_radius` is the box radius the blur actually uses rather
/// than the requested radius: the blur quantizes to it, so two radii that round
/// together really do produce the same blur.
#[derive(Clone, Copy, PartialEq, Eq)]
struct GlowShape {
    source_edge: u32,
    mark_width: u32,
    mark_height: u32,
    padding: u32,
    blur_radius: usize,
}

thread_local! {
    /// Blurred glow masks, keyed by the shape that produced them.
    static GLOWS: RefCell<Vec<(GlowShape, Mask)>> = const { RefCell::new(Vec::new()) };
}

/// Glow masks retained at once.
///
/// Each is one `f32` per pixel — a hero mark's glow is around half a megabyte —
/// so this stays small. A surface draws the mark at one or two sizes, and the
/// reveal animation asks for the same shape every frame.
const GLOW_CACHE_LIMIT: usize = 3;

/// Builds the blurred coverage for one glow shape, in mask-local coordinates.
///
/// The mark sits at `(padding, padding)` and the mask extends `padding` beyond
/// it on every side, which is the room the blur needs to fall off in.
fn glow_coverage(image: &Image, shape: GlowShape, radius: f32) -> Option<Mask> {
    let padding = shape.padding as f32;
    let mark_width = shape.mark_width as f32;
    let mark_height = shape.mark_height as f32;
    let width = shape.mark_width + shape.padding * 2;
    let height = shape.mark_height + shape.padding * 2;

    let mut coverage = Vec::with_capacity((width as usize) * (height as usize));
    for y in 0..height {
        let v = (y as f32 + 0.5 - padding) / mark_height;
        for x in 0..width {
            let u = (x as f32 + 0.5 - padding) / mark_width;
            let inside = (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v);
            coverage.push(if inside {
                f32::from(image.sample_bilinear(u, v).alpha) / 255.0
            } else {
                0.0
            });
        }
    }

    let mut mask = Mask::from_coverage(0, 0, width, height, coverage)?;
    mask.blur(radius);
    Some(mask)
}

/// Blurs the raster's alpha channel into a glow behind `bounds`.
///
/// The mask is retained across frames. A reveal moves and resizes the mark by
/// well under a pixel per frame while the blur spreads its alpha over tens of
/// pixels, so rebuilding the mask for that difference costs about 2 ms a frame
/// and changes almost nothing: the mark is placed on whole pixels here and the
/// retained mask is reused, which `a_retained_glow_matches_one_built_in_place`
/// holds to a bounded difference. See `docs/RENDERER.md`.
fn draw_raster_glow(
    canvas: &mut Canvas,
    image: &Image,
    bounds: Rect,
    radius: f32,
    style: MarkStyle,
    glow: &Paint,
) {
    let padding = (radius * 1.5).ceil().max(1.0);
    let shape = GlowShape {
        source_edge: image.width(),
        mark_width: bounds.width().round().max(1.0) as u32,
        mark_height: bounds.height().round().max(1.0) as u32,
        padding: padding as u32,
        blur_radius: Mask::blur_box_radius(radius),
    };
    let origin_x = bounds.left.round() as i32 - shape.padding as i32;
    let origin_y = bounds.top.round() as i32 - shape.padding as i32;

    GLOWS.with(|cache| {
        let mut entries = cache.borrow_mut();
        if !entries.iter().any(|(held, _)| *held == shape) {
            let Some(mask) = glow_coverage(image, shape, radius) else {
                return;
            };
            // A surface settles on a couple of shapes; past that, start over
            // rather than retain buffers this size that nothing is asking for.
            if entries.len() >= GLOW_CACHE_LIMIT {
                entries.clear();
            }
            entries.push((shape, mask));
        }
        let Some((_, mask)) = entries.iter_mut().find(|(held, _)| *held == shape) else {
            return;
        };
        mask.reposition(origin_x, origin_y);
        for _ in 0..style.glow_passes {
            canvas.fill_mask(mask, glow);
        }
    });
}

#[cfg(test)]
mod tests;
