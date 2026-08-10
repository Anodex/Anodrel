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
use std::rc::Rc;
use std::sync::OnceLock;

use anodrel_canvas::{Bevel, Canvas, Color, Image, Mask, Paint, Path, Point, Rect, Stop, point};

use crate::palette;

/// The authored mark, as straight-alpha `B, G, R, A` at [`RASTER_SIDE`] square.
///
/// This is the brand asset itself, not a reconstruction of it. It is stored
/// pre-decoded so the platform ships no image decoder and takes no dependency
/// to display its own logo. See `assets/README.md` for provenance and for the
/// step that regenerates it.
static MARK_BYTES: &[u8] = include_bytes!("../assets/mark-512.bgra");

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
fn scaled_raster(edge: f32) -> Option<Rc<Image>> {
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

/// One of the four pieces the mark is cut into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    /// The peak of the `A`.
    Apex,
    /// The descending stroke on the left.
    LeftLeg,
    /// The descending stroke on the right.
    RightLeg,
    /// The chevron forming the crossbar.
    Crossbar,
}

impl Piece {
    /// Every piece, in painting order from back to front.
    pub const ALL: [Self; 4] = [Self::LeftLeg, Self::RightLeg, Self::Crossbar, Self::Apex];

    /// Returns the piece's outline in the unit square.
    ///
    /// The mark fills the square exactly: the apex touches the top edge and the
    /// legs touch the bottom and both sides. Callers position it by choosing
    /// bounds, never by editing these numbers.
    #[must_use]
    pub fn unit_path(self) -> Path {
        match self {
            Self::Apex => Path::polygon([
                point(0.5000, 0.0000),
                point(0.7090, 0.4180),
                point(0.5580, 0.4320),
                point(0.5000, 0.2820),
                point(0.4420, 0.4320),
                point(0.2910, 0.4180),
            ]),
            Self::LeftLeg => Path::polygon(LEFT_LEG.map(|(x, y)| point(x, y))),
            Self::RightLeg => Path::polygon(mirrored(&LEFT_LEG)),
            Self::Crossbar => Path::polygon([
                point(0.5000, 0.6520),
                point(0.6970, 1.0000),
                point(0.6040, 1.0000),
                point(0.5000, 0.8000),
                point(0.3960, 1.0000),
                point(0.3030, 1.0000),
            ]),
        }
    }
}

/// The left leg, wound clockwise from its upper outer corner.
///
/// The final pair is the chamfer that blunts the outer foot, matching the way
/// the apex and crossbar terminate.
const LEFT_LEG: [(f32, f32); 5] = [
    (0.2610, 0.4780),
    (0.4112, 0.4920),
    (0.2150, 1.0000),
    (0.0750, 1.0000),
    (0.0300, 0.9400),
];

/// Mirrors a contour about the vertical centre line, preserving its winding.
fn mirrored(points: &[(f32, f32)]) -> Vec<Point> {
    points
        .iter()
        .rev()
        .map(|(x, y)| point(1.0 - x, *y))
        .collect()
}

/// Returns every piece fitted to `bounds`.
#[must_use]
pub fn pieces(bounds: Rect) -> Vec<(Piece, Path)> {
    Piece::ALL
        .iter()
        .map(|piece| (*piece, piece.unit_path().fit_unit_square(bounds)))
        .collect()
}

/// Returns all four pieces as one multi-contour path, fitted to `bounds`.
///
/// This is the shape to glow or shadow: one mask covering the whole mark rather
/// than four overlapping ones.
#[must_use]
pub fn silhouette(bounds: Rect) -> Path {
    let mut path = Path::new();
    for piece in Piece::ALL {
        for contour in piece.unit_path().fit_unit_square(bounds).contours() {
            path.push_contour(contour.clone());
        }
    }
    path
}

/// Returns the violet-to-blue face gradient spanning `bounds` horizontally.
#[must_use]
pub fn face_paint(bounds: Rect) -> Paint {
    Paint::linear(
        point(bounds.left, 0.0),
        point(bounds.right, 0.0),
        palette::mark_ramp()
            .map(|(position, color)| Stop::new(position, color))
            .to_vec(),
    )
}

/// Returns the vertical shading laid over each piece to give it depth.
///
/// The face gradient runs horizontally, so a separate low-opacity vertical pass
/// is what keeps a tall piece from looking like flat tape.
#[must_use]
pub fn depth_paint(bounds: Rect) -> Paint {
    Paint::linear(
        point(0.0, bounds.top),
        point(0.0, bounds.bottom),
        vec![
            Stop::new(0.0, Color::WHITE.with_alpha(18)),
            Stop::new(0.45, Color::TRANSPARENT),
            Stop::new(1.0, Color::BLACK.with_alpha(48)),
        ],
    )
}

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
mod tests {
    use super::{
        GLOW_CACHE_LIMIT, GLOWS, MarkStyle, Piece, draw, draw_glow_layer, face_paint, glow_paint,
        pieces, scaled_raster, silhouette,
    };
    use anodrel_canvas::{Canvas, Color, Mask, Rect, point};

    const UNIT: Rect = Rect::new(0.0, 0.0, 1.0, 1.0);

    /// Draws the glow the way it was drawn before the mask was retained:
    /// coverage sampled at the exact sub-pixel bounds, blurred, then composited.
    ///
    /// This is the reference the retained mask is held against, so it stays
    /// spelled out here rather than sharing code with the path under test.
    fn glow_built_in_place(canvas: &mut Canvas, bounds: Rect, style: MarkStyle) {
        let image = scaled_raster(bounds.width().max(bounds.height())).expect("the mark decodes");
        let paint = glow_paint(bounds).scale_alpha(style.opacity);
        let radius = bounds.width() * style.glow_ratio;
        let padding = (radius * 1.5).ceil().max(1.0);
        let origin_x = (bounds.left - padding).floor() as i32;
        let origin_y = (bounds.top - padding).floor() as i32;
        let width = (bounds.width() + padding * 2.0).ceil().max(1.0) as u32;
        let height = (bounds.height() + padding * 2.0).ceil().max(1.0) as u32;

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

        let mut mask = Mask::from_coverage(origin_x, origin_y, width, height, coverage)
            .expect("the coverage matches its size");
        mask.blur(radius);
        for _ in 0..style.glow_passes {
            canvas.fill_mask(&mask, &paint);
        }
    }

    /// Largest per-channel difference between two canvases of the same size.
    fn largest_channel_difference(left: &Canvas, right: &Canvas) -> i16 {
        let mut worst = 0;
        for y in 0..left.height() as i32 {
            for x in 0..left.width() as i32 {
                let (a, b) = (left.pixel(x, y), right.pixel(x, y));
                for (from, to) in [
                    (a.red, b.red),
                    (a.green, b.green),
                    (a.blue, b.blue),
                    (a.alpha, b.alpha),
                ] {
                    worst = worst.max((i16::from(from) - i16::from(to)).abs());
                }
            }
        }
        worst
    }

    /// Largest channel difference the retained glow may show against one built
    /// at the exact sub-pixel bounds, out of 255.
    ///
    /// The retained mask is placed on whole pixels and sized to whole pixels,
    /// so it can sit up to half a pixel from where an exactly built one would.
    /// The mark's alpha is spread over a 48-pixel blur before it is composited,
    /// which is what turns that half pixel into a handful of levels rather than
    /// a visible shift. Measured across quarter-pixel placements the two agree
    /// exactly on the grid and differ by at most 7 at the half-pixel worst
    /// case, so this is that with a step to spare.
    const GLOW_TOLERANCE: i16 = 8;

    #[test]
    fn a_retained_glow_matches_one_built_in_place() {
        let style = MarkStyle::hero();
        // Every quarter-pixel placement, including the half-pixel worst case.
        for step in 0..8_u8 {
            // Both the position and the size are taken off the pixel grid, so
            // the retained mask is rounded on every axis it can be rounded on.
            let offset = f32::from(step) / 4.0;
            let (left, top, edge) = (46.0 + offset, 38.0 + offset, 220.0 + offset);
            let bounds = Rect::new(left, top, left + edge, top + edge);
            let mut retained = Canvas::new(320, 300);
            let mut in_place = Canvas::new(320, 300);
            // Composited over the opaque backdrop it is drawn over in use. On a
            // transparent canvas the fully transparent pixels around the glow
            // keep whatever colour was blended at zero alpha, which is
            // invisible but not equal.
            retained.clear(Color::rgb(11, 13, 22));
            in_place.clear(Color::rgb(11, 13, 22));

            GLOWS.with(|cache| cache.borrow_mut().clear());
            draw_glow_layer(&mut retained, bounds, style);
            glow_built_in_place(&mut in_place, bounds, style);

            let worst = largest_channel_difference(&retained, &in_place);
            assert!(
                worst <= GLOW_TOLERANCE,
                "at a {offset} pixel offset the retained glow differs by {worst} of 255"
            );
        }
    }

    #[test]
    fn a_reveal_reuses_one_retained_glow() {
        // A reveal changes the mark's size by well under a pixel per frame.
        // Every one of those frames has to land on the same retained mask, or
        // the retention saves nothing.
        GLOWS.with(|cache| cache.borrow_mut().clear());
        let mut canvas = Canvas::new(320, 300);
        for frame in 0..40_u8 {
            let scale = 0.999 + 0.001 * (f32::from(frame) / 39.0);
            let edge = 220.0 * scale;
            let bounds = Rect::new(46.0, 38.0, 46.0 + edge, 38.0 + edge);
            draw_glow_layer(&mut canvas, bounds, MarkStyle::hero());
        }
        assert_eq!(GLOWS.with(|cache| cache.borrow().len()), 1);
    }

    #[test]
    fn retained_glows_stay_bounded() {
        GLOWS.with(|cache| cache.borrow_mut().clear());
        let mut canvas = Canvas::new(900, 900);
        for step in 0..12_u8 {
            let edge = 200.0 + f32::from(step) * 40.0;
            let bounds = Rect::new(10.0, 10.0, 10.0 + edge, 10.0 + edge);
            draw_glow_layer(&mut canvas, bounds, MarkStyle::hero());
            assert!(GLOWS.with(|cache| cache.borrow().len()) <= GLOW_CACHE_LIMIT);
        }
    }

    #[test]
    fn every_piece_stays_inside_the_unit_square() {
        for piece in Piece::ALL {
            let bounds = piece.unit_path().bounds();
            assert!(
                bounds.left >= 0.0 && bounds.top >= 0.0,
                "{piece:?} escapes the top-left"
            );
            assert!(
                bounds.right <= 1.0 && bounds.bottom <= 1.0,
                "{piece:?} escapes the bottom-right"
            );
        }
    }

    #[test]
    fn the_legs_mirror_each_other_about_the_centre_line() {
        let left = Piece::LeftLeg.unit_path().bounds();
        let right = Piece::RightLeg.unit_path().bounds();
        assert!((left.left - (1.0 - right.right)).abs() < 1e-4);
        assert!((left.right - (1.0 - right.left)).abs() < 1e-4);
        assert!((left.top - right.top).abs() < 1e-4);
        assert!((left.bottom - right.bottom).abs() < 1e-4);
    }

    #[test]
    fn the_apex_and_crossbar_are_symmetric_about_the_centre_line() {
        for piece in [Piece::Apex, Piece::Crossbar] {
            let bounds = piece.unit_path().bounds();
            let center = (bounds.left + bounds.right) / 2.0;
            assert!(
                (center - 0.5).abs() < 1e-4,
                "{piece:?} is not centred: {center}"
            );
        }
    }

    #[test]
    fn the_legs_and_crossbar_share_one_baseline() {
        let baseline = Piece::LeftLeg.unit_path().bounds().bottom;
        assert!((Piece::RightLeg.unit_path().bounds().bottom - baseline).abs() < 1e-4);
        assert!((Piece::Crossbar.unit_path().bounds().bottom - baseline).abs() < 1e-4);
    }

    #[test]
    fn the_pieces_are_separated_by_visible_gaps() {
        let apex_bottom = Piece::Apex.unit_path().bounds().bottom;
        let leg_top = Piece::LeftLeg.unit_path().bounds().top;
        assert!(
            leg_top > apex_bottom,
            "apex ends at {apex_bottom} but the leg starts at {leg_top}"
        );
    }

    #[test]
    fn the_silhouette_carries_one_contour_per_piece() {
        assert_eq!(silhouette(UNIT).contours().len(), Piece::ALL.len());
    }

    #[test]
    fn fitting_places_the_mark_inside_its_target() {
        let target = Rect::new(100.0, 40.0, 300.0, 240.0);
        for (piece, path) in pieces(target) {
            let bounds = path.bounds();
            assert!(
                bounds.left >= target.left - 0.01 && bounds.right <= target.right + 0.01,
                "{piece:?} escapes horizontally"
            );
            assert!(
                bounds.top >= target.top - 0.01 && bounds.bottom <= target.bottom + 0.01,
                "{piece:?} escapes vertically"
            );
        }
    }

    #[test]
    fn the_face_gradient_runs_violet_on_the_left_and_blue_on_the_right() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let paint = face_paint(bounds);
        let left = paint.sample(point(0.0, 50.0));
        let right = paint.sample(point(100.0, 50.0));
        assert!(left.red > right.red, "the left end should be violet");
        assert!(right.blue > right.red, "the right end should be blue");
    }

    #[test]
    fn drawing_the_mark_paints_inside_its_bounds_and_leaves_the_corners_alone() {
        let mut canvas = Canvas::new(240, 240);
        canvas.clear(Color::BLACK);
        draw(
            &mut canvas,
            Rect::new(20.0, 20.0, 220.0, 220.0),
            MarkStyle::compact(),
        );
        // The apex sits on the centre line near the top of the mark.
        assert_ne!(canvas.pixel(120, 60), Color::BLACK);
        // The unit square's corners are outside every piece.
        assert_eq!(canvas.pixel(2, 2), Color::BLACK);
        assert_eq!(canvas.pixel(237, 2), Color::BLACK);
    }

    #[test]
    fn a_fully_faded_mark_draws_nothing() {
        let mut canvas = Canvas::new(120, 120);
        canvas.clear(Color::BLACK);
        draw(
            &mut canvas,
            Rect::new(10.0, 10.0, 110.0, 110.0),
            MarkStyle::hero().with_opacity(0.0),
        );
        assert_eq!(canvas.pixel(60, 40), Color::BLACK);
    }

    #[test]
    fn an_empty_target_is_ignored() {
        let mut canvas = Canvas::new(32, 32);
        canvas.clear(Color::BLACK);
        draw(
            &mut canvas,
            Rect::new(10.0, 10.0, 10.0, 10.0),
            MarkStyle::hero(),
        );
        assert_eq!(canvas.pixel(10, 10), Color::BLACK);
    }

    #[test]
    fn the_authored_asset_loads_at_its_declared_size() {
        let image = super::raster().expect("the embedded asset matches RASTER_SIDE");
        assert_eq!(image.width(), super::RASTER_SIDE);
        assert_eq!(image.height(), super::RASTER_SIDE);
    }

    #[test]
    fn the_authored_asset_is_a_clean_cut_out() {
        let image = super::raster().expect("asset loads");
        // Corners must be empty, or the mark would sit on a plate.
        for (x, y) in [(0, 0), (511, 0), (0, 511), (511, 511)] {
            assert_eq!(image.pixel(x, y).alpha, 0, "corner ({x},{y}) is not clear");
        }
        // No baked glow: the negative space inside the A stays empty, so the
        // draw-time bloom cannot double up.
        assert_eq!(
            image.pixel(256, 235).alpha,
            0,
            "inner negative space is not clear"
        );
    }

    #[test]
    fn the_artwork_reaches_every_edge_of_the_asset() {
        // The asset is cropped square to its artwork, which is what lets the
        // raster and the geometry share one set of bounds.
        let image = super::raster().expect("asset loads");
        let (left, top, right, bottom) =
            image.opaque_bounds(3).expect("the asset contains artwork");
        assert!(left <= 2 && top <= 2, "artwork is inset at ({left},{top})");
        assert!(
            right >= super::RASTER_SIDE - 2 && bottom >= super::RASTER_SIDE - 2,
            "artwork stops short at ({right},{bottom})"
        );
    }

    #[test]
    fn the_raster_and_the_geometry_cover_the_same_bounds() {
        // Placement must not shift when the renderer crosses RASTER_MIN_EDGE.
        let painted_extent = |edge: f32| {
            let side = (edge as u32) + 40;
            let mut canvas = Canvas::new(side, side);
            canvas.clear(Color::BLACK);
            let bounds = Rect::new(20.0, 20.0, 20.0 + edge, 20.0 + edge);
            draw(&mut canvas, bounds, MarkStyle::compact());
            let lit: Vec<(i32, i32)> = (0..side as i32)
                .flat_map(|y| (0..side as i32).map(move |x| (x, y)))
                .filter(|(x, y)| canvas.pixel(*x, *y) != Color::BLACK)
                .collect();
            let left = lit.iter().map(|(x, _)| *x).min().expect("drew");
            let right = lit.iter().map(|(x, _)| *x).max().expect("drew");
            (left, right)
        };

        // Just below and just above the threshold, normalised to the same edge.
        let (geometry_left, geometry_right) = painted_extent(super::RASTER_MIN_EDGE - 2.0);
        let (raster_left, raster_right) = painted_extent(super::RASTER_MIN_EDGE + 2.0);
        assert!(
            (geometry_left - raster_left).abs() <= 3,
            "left edge jumps: geometry {geometry_left}, raster {raster_left}"
        );
        assert!(
            (geometry_right - raster_right).abs() <= 5,
            "right edge jumps: geometry {geometry_right}, raster {raster_right}"
        );
    }

    #[test]
    fn a_hero_sized_mark_carries_the_brand_gradient() {
        let mut canvas = Canvas::new(320, 320);
        canvas.clear(Color::BLACK);
        draw(
            &mut canvas,
            Rect::new(20.0, 20.0, 300.0, 300.0),
            MarkStyle::compact(),
        );
        // Sample the two legs: violet on the left, blue on the right.
        let left_leg = canvas.pixel(90, 250);
        let right_leg = canvas.pixel(240, 250);
        assert!(
            left_leg.red > left_leg.green + 40,
            "left leg is not violet: {left_leg:?}"
        );
        assert!(
            right_leg.blue > right_leg.red + 40,
            "right leg is not blue: {right_leg:?}"
        );
    }
}
