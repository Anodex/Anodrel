//! The coverage rasterizer, the pixel buffer, and the blurred coverage mask.

use crate::color::Color;
use crate::geometry::{Point, Rect};
use crate::paint::Paint;
use crate::path::Path;

/// Vertical samples taken per pixel row when computing edge coverage.
///
/// Horizontal coverage is exact — spans carry fractional endpoints — so only
/// the vertical axis is sampled. Eight rows put the residual error below what
/// an 8-bit channel can represent on all but the shallowest edges.
const SUBSAMPLES: usize = 8;

/// Coverage below this is dropped; it cannot change an 8-bit channel.
const COVERAGE_EPSILON: f32 = 1.0 / 512.0;

/// A 32-bit ARGB pixel buffer that shapes are composited into.
///
/// The canvas is a plain buffer with no device affinity: the same drawing code
/// runs under a unit test and behind a native window. Presenting it is the
/// host's job.
#[derive(Clone)]
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl Canvas {
    /// Builds a fully transparent canvas.
    ///
    /// Surfaces that own their whole area call [`Canvas::clear`] first; starting
    /// transparent is what lets a canvas be used as a source with real alpha,
    /// such as a window icon.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize)],
        }
    }

    /// Returns the width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the whole canvas as a rectangle.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width as f32, self.height as f32)
    }

    /// Returns the packed `0xAARRGGBB` pixels in row-major order.
    ///
    /// On a little-endian target the bytes are already `B, G, R, A`, so this can
    /// be handed to a 32-bit `BI_RGB` bitmap without a conversion pass.
    #[must_use]
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// Returns the colour at a pixel, or transparent when out of bounds.
    ///
    /// Reading back matters for text: the host draws glyphs with an API that has
    /// no alpha, so a fading label resolves its colour against the exact pixels
    /// already rendered underneath it.
    #[must_use]
    pub fn pixel(&self, x: i32, y: i32) -> Color {
        match self.index(x, y) {
            Some(index) => Color::from_argb(self.pixels[index]),
            None => Color::TRANSPARENT,
        }
    }

    /// Returns the colour at a point, clamped to the canvas edges.
    #[must_use]
    pub fn sample(&self, at: Point) -> Color {
        if self.width == 0 || self.height == 0 {
            return Color::TRANSPARENT;
        }
        let x = (at.x as i32).clamp(0, self.width as i32 - 1);
        let y = (at.y as i32).clamp(0, self.height as i32 - 1);
        self.pixel(x, y)
    }

    /// Replaces every pixel with an opaque colour.
    pub fn clear(&mut self, color: Color) {
        self.pixels.fill(color.with_alpha(255).to_argb());
    }

    /// Copies another canvas of the same size over this one.
    ///
    /// Returns `false` when the sizes differ, leaving this canvas untouched.
    /// This is how an expensive but unchanging layer — a gradient backdrop
    /// redrawn every animation frame, say — is computed once and then reused.
    pub fn copy_from(&mut self, source: &Self) -> bool {
        if source.width != self.width || source.height != self.height {
            return false;
        }
        self.pixels.copy_from_slice(&source.pixels);
        true
    }

    /// Copies a rectangle from a same-sized canvas, replacing those pixels.
    ///
    /// Restoring one region from a cached layer is what makes an animation that
    /// only touches part of the surface cost only that part.
    pub fn copy_region_from(&mut self, source: &Self, region: Rect) -> bool {
        if source.width != self.width || source.height != self.height {
            return false;
        }
        let clip = region.intersect(self.bounds());
        if clip.is_empty() {
            return true;
        }
        let left = clip.left.floor().max(0.0) as usize;
        let right = (clip.right.ceil() as usize).min(self.width as usize);
        let top = clip.top.floor().max(0.0) as usize;
        let bottom = (clip.bottom.ceil() as usize).min(self.height as usize);
        for row in top..bottom {
            let start = row * (self.width as usize) + left;
            let end = row * (self.width as usize) + right;
            self.pixels[start..end].copy_from_slice(&source.pixels[start..end]);
        }
        true
    }

    /// Composites another canvas at an offset, scaled by `opacity`.
    ///
    /// Layers are pre-rendered at their final size, so this is a straight
    /// per-pixel blend with no resampling — cheap enough to run every frame.
    pub fn draw_canvas(&mut self, source: &Self, x: i32, y: i32, opacity: f32) {
        self.draw_canvas_clipped(source, x, y, opacity, self.bounds());
    }

    /// Composites another canvas at an offset inside one explicit destination clip.
    ///
    /// The clip is intersected with this canvas and rounded out to whole target
    /// pixels. It is an argument to this one operation, never retained drawing
    /// state, so callers can compose a pre-rendered layer into a viewport
    /// without affecting later drawing.
    pub fn draw_canvas_clipped(&mut self, source: &Self, x: i32, y: i32, opacity: f32, clip: Rect) {
        let opacity = opacity.clamp(0.0, 1.0);
        let clip = clip.intersect(self.bounds());
        if opacity <= 0.0 || clip.is_empty() {
            return;
        }
        let clip_left = clip.left.floor() as i32;
        let clip_right = clip.right.ceil() as i32;
        let clip_top = clip.top.floor() as i32;
        let clip_bottom = clip.bottom.ceil() as i32;
        let column_start =
            (i64::from(clip_left) - i64::from(x)).clamp(0, i64::from(source.width)) as u32;
        let column_end =
            (i64::from(clip_right) - i64::from(x)).clamp(0, i64::from(source.width)) as u32;
        let row_start =
            (i64::from(clip_top) - i64::from(y)).clamp(0, i64::from(source.height)) as u32;
        let row_end =
            (i64::from(clip_bottom) - i64::from(y)).clamp(0, i64::from(source.height)) as u32;

        for row in row_start..row_end {
            let source_row = (row as usize) * (source.width as usize);
            let target_y = (y + row as i32) as u32;
            for column in column_start..column_end {
                let sample = Color::from_argb(source.pixels[source_row + column as usize]);
                if sample.alpha == 0 {
                    continue;
                }
                let target_x = (x + column as i32) as u32;
                blend_into(
                    &mut self.pixels,
                    self.width,
                    target_x,
                    target_y,
                    sample,
                    opacity,
                );
            }
        }
    }

    /// Composites a shape using the non-zero winding rule with anti-aliased edges.
    pub fn fill_path(&mut self, path: &Path, paint: &Paint) {
        let (width, height) = (self.width, self.height);
        let pixels = &mut self.pixels;
        // Most fills are flat. Resolving that once keeps the per-pixel work to
        // a blend, instead of re-dispatching the paint for every sample.
        if let Paint::Solid(color) = paint {
            let color = *color;
            rasterize(path, width, height, |x, y, coverage| {
                blend_into(pixels, width, x, y, color, coverage);
            });
            return;
        }
        rasterize(path, width, height, |x, y, coverage| {
            let center = Point::new(x as f32 + 0.5, y as f32 + 0.5);
            blend_into(pixels, width, x, y, paint.sample(center), coverage);
        });
    }

    /// Composites an axis-aligned rectangle.
    pub fn fill_rect(&mut self, rect: Rect, paint: &Paint) {
        self.fill_path(&Path::rect(rect), paint);
    }

    /// Composites a rectangle with rounded corners.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, paint: &Paint) {
        self.fill_path(&Path::rounded_rect(rect, radius), paint);
    }

    /// Composites a circle.
    pub fn fill_circle(&mut self, center: Point, radius: f32, paint: &Paint) {
        self.fill_path(&Path::circle(center, radius), paint);
    }

    /// Draws an outline of `width` pixels centred on the path's edges.
    pub fn stroke_path(&mut self, path: &Path, width: f32, paint: &Paint) {
        let half = width / 2.0;
        let outer = path.inset(-half);
        let inner = path.inset(half);
        let mut ring = outer;
        for contour in inner.contours() {
            let mut reversed = contour.clone();
            reversed.reverse();
            ring.push_contour(reversed);
        }
        self.fill_path(&ring, paint);
    }

    /// Draws a rounded-rectangle outline.
    pub fn stroke_rounded_rect(&mut self, rect: Rect, radius: f32, width: f32, paint: &Paint) {
        self.stroke_path(&Path::rounded_rect(rect, radius), width, paint);
    }

    /// Draws a straight line of the given width.
    pub fn draw_line(&mut self, from: Point, to: Point, width: f32, paint: &Paint) {
        let Some(direction) = from.to(to).normalized() else {
            return;
        };
        let normal = Point::new(-direction.y, direction.x).scale(width / 2.0);
        self.fill_path(
            &Path::polygon([
                from.offset(normal.x, normal.y),
                to.offset(normal.x, normal.y),
                to.offset(-normal.x, -normal.y),
                from.offset(-normal.x, -normal.y),
            ]),
            paint,
        );
    }

    /// Draws a connected run of line segments with rounded joins and caps.
    ///
    /// Each segment is stroked independently and a disc is placed at every
    /// interior vertex. That is cheaper than mitring an open path and, at the
    /// weights icons use, indistinguishable from it.
    pub fn draw_polyline(&mut self, points: &[Point], width: f32, paint: &Paint) {
        if points.len() < 2 || width <= 0.0 {
            return;
        }
        for pair in points.windows(2) {
            self.draw_line(pair[0], pair[1], width, paint);
        }
        if points.len() > 2 {
            for vertex in &points[1..points.len() - 1] {
                self.fill_circle(*vertex, width / 2.0, paint);
            }
        }
    }

    /// Composites a blurred coverage mask.
    pub fn fill_mask(&mut self, mask: &Mask, paint: &Paint) {
        let solid = match paint {
            Paint::Solid(color) => Some(*color),
            _ => None,
        };
        // Rows and columns are clamped once rather than tested per pixel: a
        // mask is usually mostly on-canvas, and glyph masks are composited many
        // times per frame.
        let row_start = (-mask.origin_y).max(0) as u32;
        let row_end = mask
            .height
            .min((self.height as i32 - mask.origin_y).max(0) as u32);
        let column_start = (-mask.origin_x).max(0) as u32;
        let column_end = mask
            .width
            .min((self.width as i32 - mask.origin_x).max(0) as u32);

        for row in row_start..row_end {
            let y = (mask.origin_y + row as i32) as u32;
            let stride = (row as usize) * (mask.width as usize);
            for column in column_start..column_end {
                let coverage = mask.coverage[stride + column as usize];
                if coverage <= COVERAGE_EPSILON {
                    continue;
                }
                let x = (mask.origin_x + column as i32) as u32;
                let color = solid
                    .unwrap_or_else(|| paint.sample(Point::new(x as f32 + 0.5, y as f32 + 0.5)));
                blend_into(&mut self.pixels, self.width, x, y, color, coverage.min(1.0));
            }
        }
    }

    /// Draws a soft glow around a shape.
    ///
    /// The shape is rasterized into a tightly sized mask, blurred, and then
    /// composited. Repeating the composite deepens the falloff without widening
    /// it, which is what separates a glow from a flat halo.
    pub fn draw_glow(&mut self, path: &Path, radius: f32, passes: u32, paint: &Paint) {
        if radius <= 0.0 || passes == 0 {
            return;
        }
        let mut mask = Mask::for_path(path, radius * 1.5);
        mask.blur(radius);
        for _ in 0..passes {
            self.fill_mask(&mask, paint);
        }
    }

    /// Draws a blurred shadow offset from a shape.
    pub fn draw_shadow(&mut self, path: &Path, offset: Point, radius: f32, color: Color) {
        let shifted = path.translate(offset.x, offset.y);
        let mut mask = Mask::for_path(&shifted, radius * 1.5);
        mask.blur(radius);
        self.fill_mask(&mask, &Paint::solid(color));
    }

    /// Composites a single colour over one pixel.
    ///
    /// Exposed within the crate so image compositing shares exactly the same
    /// blend as vector filling.
    pub(crate) fn blend_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        blend_into(&mut self.pixels, self.width, x, y, color, 1.0);
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some((y as usize) * (self.width as usize) + (x as usize))
    }
}

/// An anti-aliased coverage buffer that can be blurred and reused.
///
/// A mask is positioned by its origin in canvas space and sized to the geometry
/// it holds, so a glow does not pay for the whole surface.
#[derive(Clone)]
pub struct Mask {
    origin_x: i32,
    origin_y: i32,
    width: u32,
    height: u32,
    coverage: Vec<f32>,
}

impl Mask {
    /// Builds an empty mask covering a region of canvas space.
    #[must_use]
    pub fn new(origin_x: i32, origin_y: i32, width: u32, height: u32) -> Self {
        Self {
            origin_x,
            origin_y,
            width,
            height,
            coverage: vec![0.0; (width as usize) * (height as usize)],
        }
    }

    /// Wraps externally produced coverage, such as glyph coverage from a
    /// platform text engine.
    ///
    /// Returns `None` when `coverage` is not exactly `width * height` long.
    /// This is the seam that lets a host render text through its own font
    /// stack and still composite it with canvas paints.
    #[must_use]
    pub fn from_coverage(
        origin_x: i32,
        origin_y: i32,
        width: u32,
        height: u32,
        coverage: Vec<f32>,
    ) -> Option<Self> {
        if coverage.len() != (width as usize) * (height as usize) {
            return None;
        }
        Some(Self {
            origin_x,
            origin_y,
            width,
            height,
            coverage,
        })
    }

    /// Returns a copy positioned at a new origin in canvas space.
    #[must_use]
    pub fn positioned(&self, origin_x: i32, origin_y: i32) -> Self {
        Self {
            origin_x,
            origin_y,
            ..self.clone()
        }
    }

    /// Builds a mask sized to a path's bounds plus `padding`, and rasterizes it.
    #[must_use]
    pub fn for_path(path: &Path, padding: f32) -> Self {
        let bounds = path.bounds().inflate(padding.max(0.0));
        let origin_x = bounds.left.floor() as i32;
        let origin_y = bounds.top.floor() as i32;
        let width = (bounds.right.ceil() as i32 - origin_x).max(0) as u32;
        let height = (bounds.bottom.ceil() as i32 - origin_y).max(0) as u32;
        let mut mask = Self::new(origin_x, origin_y, width, height);
        mask.fill_path(path);
        mask
    }

    /// Returns the mask width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the mask height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the coverage at a mask-local pixel, or `0.0` when out of bounds.
    #[must_use]
    pub fn coverage_at(&self, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return 0.0;
        }
        self.coverage[(y as usize) * (self.width as usize) + (x as usize)]
    }

    /// Accumulates a shape's coverage, saturating at fully covered.
    pub fn fill_path(&mut self, path: &Path) {
        let local = path.translate(-self.origin_x as f32, -self.origin_y as f32);
        let (width, height) = (self.width, self.height);
        let coverage = &mut self.coverage;
        rasterize(&local, width, height, |x, y, value| {
            let index = (y as usize) * (width as usize) + (x as usize);
            coverage[index] = (coverage[index] + value).min(1.0);
        });
    }

    /// Blurs the coverage with three box passes, approximating a Gaussian.
    ///
    /// Three passes are the standard trade: the error against a true Gaussian
    /// falls below visual threshold while the cost stays linear in pixels rather
    /// than in radius.
    pub fn blur(&mut self, radius: f32) {
        if radius <= 0.0 || self.width == 0 || self.height == 0 {
            return;
        }
        let box_radius = ((radius / 3.0).round() as usize).max(1);
        let mut scratch = vec![0.0; self.coverage.len()];
        for _ in 0..3 {
            blur_horizontal(
                &self.coverage,
                &mut scratch,
                self.width,
                self.height,
                box_radius,
            );
            blur_vertical(
                &scratch,
                &mut self.coverage,
                self.width,
                self.height,
                box_radius,
            );
        }
    }
}

fn blur_horizontal(source: &[f32], target: &mut [f32], width: u32, height: u32, radius: usize) {
    let width = width as usize;
    let window = (radius * 2 + 1) as f32;
    if width == 0 {
        return;
    }
    for y in 0..height as usize {
        let row = y * width;
        let mut sum = source[row] * radius as f32;
        for offset in 0..=radius {
            sum += source[row + offset.min(width - 1)];
        }
        for x in 0..width {
            target[row + x] = sum / window;
            sum += source[row + (x + radius + 1).min(width - 1)];
            sum -= source[row + x.saturating_sub(radius)];
        }
    }
}

fn blur_vertical(source: &[f32], target: &mut [f32], width: u32, height: u32, radius: usize) {
    let (width, height) = (width as usize, height as usize);
    let window = (radius * 2 + 1) as f32;
    if height == 0 {
        return;
    }
    for x in 0..width {
        let mut sum = source[x] * radius as f32;
        for offset in 0..=radius {
            sum += source[offset.min(height - 1) * width + x];
        }
        for y in 0..height {
            target[y * width + x] = sum / window;
            sum += source[(y + radius + 1).min(height - 1) * width + x];
            sum -= source[y.saturating_sub(radius) * width + x];
        }
    }
}

/// Walks a path and reports per-pixel coverage in `0.0..=1.0`.
///
/// Each pixel row is sampled at [`SUBSAMPLES`] sub-scanlines. For every
/// sub-scanline the edge crossings are sorted and paired under the non-zero
/// winding rule, then the resulting spans are accumulated with exact fractional
/// endpoints. Cost is proportional to covered area rather than to canvas size.
fn rasterize(path: &Path, width: u32, height: u32, mut emit: impl FnMut(u32, u32, f32)) {
    if path.is_empty() || width == 0 || height == 0 {
        return;
    }
    let clip = Rect::new(0.0, 0.0, width as f32, height as f32);
    let bounds = path.bounds().intersect(clip);
    if bounds.is_empty() {
        return;
    }

    let x_start = bounds.left.floor().max(0.0) as u32;
    let x_end = (bounds.right.ceil().min(width as f32)) as u32;
    let y_start = bounds.top.floor().max(0.0) as u32;
    let y_end = (bounds.bottom.ceil().min(height as f32)) as u32;
    if x_end <= x_start || y_end <= y_start {
        return;
    }

    let mut coverage = vec![0.0_f32; (x_end - x_start) as usize];
    let mut crossings: Vec<(f32, i32)> = Vec::new();
    let weight = 1.0 / SUBSAMPLES as f32;

    for y in y_start..y_end {
        coverage.fill(0.0);
        for sub in 0..SUBSAMPLES {
            let sample_y = y as f32 + (sub as f32 + 0.5) / SUBSAMPLES as f32;
            crossings.clear();
            for contour in path.contours() {
                let count = contour.len();
                for index in 0..count {
                    let from = contour[index];
                    let to = contour[(index + 1) % count];
                    // A half-open vertical test counts a shared vertex once.
                    if (from.y <= sample_y) == (to.y <= sample_y) {
                        continue;
                    }
                    let travel = (sample_y - from.y) / (to.y - from.y);
                    let x = from.x + travel * (to.x - from.x);
                    crossings.push((x, if to.y > from.y { 1 } else { -1 }));
                }
            }
            if crossings.len() < 2 {
                continue;
            }
            crossings.sort_by(|left, right| left.0.total_cmp(&right.0));

            let mut winding = 0;
            let mut span_start = 0.0;
            for &(x, direction) in &crossings {
                if winding == 0 {
                    span_start = x;
                }
                winding += direction;
                if winding == 0 {
                    accumulate_span(&mut coverage, x_start, span_start, x, weight);
                }
            }
        }
        for (offset, value) in coverage.iter().enumerate() {
            let value = value.min(1.0);
            if value > COVERAGE_EPSILON {
                emit(x_start + offset as u32, y, value);
            }
        }
    }
}

/// Adds a horizontal span's exact coverage into a row accumulator.
fn accumulate_span(coverage: &mut [f32], offset: u32, from: f32, to: f32, weight: f32) {
    let limit = coverage.len() as f32;
    let from = (from - offset as f32).clamp(0.0, limit);
    let to = (to - offset as f32).clamp(0.0, limit);
    if to <= from {
        return;
    }
    let first = (from.floor() as usize).min(coverage.len() - 1);
    let last = ((to.ceil() as usize).saturating_sub(1)).min(coverage.len() - 1);
    if first == last {
        coverage[first] += (to - from) * weight;
        return;
    }
    coverage[first] += ((first + 1) as f32 - from) * weight;
    for value in &mut coverage[first + 1..last] {
        *value += weight;
    }
    coverage[last] += (to - last as f32) * weight;
}

/// Composites one source sample over a pixel using source-over.
///
/// A surface that owns its background is opaque almost everywhere, so that case
/// takes an integer path: source-over against an opaque destination is a plain
/// interpolation, with no alpha to solve for and no division. The general form
/// below is only needed when the destination is itself translucent, which
/// happens when a canvas is being used as a source — a window icon, say.
#[inline]
fn blend_into(pixels: &mut [u32], width: u32, x: u32, y: u32, color: Color, coverage: f32) {
    let source_alpha = (f32::from(color.alpha) / 255.0) * coverage;
    if source_alpha <= 0.0 {
        return;
    }
    let index = (y as usize) * (width as usize) + (x as usize);
    let packed = pixels[index];

    if packed >> 24 == 0xFF {
        // 0..=256 keeps the reciprocal a shift rather than a divide.
        let weight = ((source_alpha * 256.0) as u32).min(256);
        let inverse = 256 - weight;
        let mix = |source: u8, destination: u32| {
            ((u32::from(source) * weight + destination * inverse) >> 8) & 0xFF
        };
        pixels[index] = 0xFF00_0000
            | (mix(color.red, (packed >> 16) & 0xFF) << 16)
            | (mix(color.green, (packed >> 8) & 0xFF) << 8)
            | mix(color.blue, packed & 0xFF);
        return;
    }

    let backdrop = Color::from_argb(packed);
    let backdrop_alpha = f32::from(backdrop.alpha) / 255.0;
    let result_alpha = source_alpha + backdrop_alpha * (1.0 - source_alpha);
    if result_alpha <= 0.0 {
        pixels[index] = 0;
        return;
    }
    let channel = |source: u8, destination: u8| {
        let blended = f32::from(source) * source_alpha
            + f32::from(destination) * backdrop_alpha * (1.0 - source_alpha);
        (blended / result_alpha).round().clamp(0.0, 255.0) as u8
    };
    pixels[index] = Color::rgba(
        channel(color.red, backdrop.red),
        channel(color.green, backdrop.green),
        channel(color.blue, backdrop.blue),
        (result_alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    )
    .to_argb();
}

#[cfg(test)]
mod tests {
    use super::{Canvas, Mask};
    use crate::color::Color;
    use crate::geometry::{Rect, point};
    use crate::paint::Paint;
    use crate::path::Path;

    #[test]
    fn a_pixel_aligned_rectangle_fills_exactly_its_pixels() {
        let mut canvas = Canvas::new(8, 8);
        canvas.clear(Color::BLACK);
        canvas.fill_rect(Rect::new(2.0, 2.0, 6.0, 6.0), &Paint::solid(Color::WHITE));
        assert_eq!(canvas.pixel(1, 3), Color::BLACK);
        assert_eq!(canvas.pixel(2, 3), Color::WHITE);
        assert_eq!(canvas.pixel(5, 3), Color::WHITE);
        assert_eq!(canvas.pixel(6, 3), Color::BLACK);
    }

    #[test]
    fn a_half_covered_pixel_receives_half_coverage() {
        let mut canvas = Canvas::new(4, 4);
        canvas.clear(Color::BLACK);
        canvas.fill_rect(Rect::new(0.0, 0.0, 1.5, 4.0), &Paint::solid(Color::WHITE));
        assert_eq!(canvas.pixel(0, 0), Color::WHITE);
        let partial = canvas.pixel(1, 0);
        assert!(
            (i16::from(partial.red) - 128).abs() <= 2,
            "expected roughly half coverage, got {partial:?}"
        );
    }

    #[test]
    fn a_new_canvas_starts_fully_transparent() {
        let canvas = Canvas::new(4, 4);
        assert_eq!(canvas.pixel(2, 2).alpha, 0);
    }

    #[test]
    fn compositing_onto_a_transparent_canvas_accumulates_alpha() {
        let mut canvas = Canvas::new(4, 4);
        canvas.fill_rect(canvas.bounds(), &Paint::solid(Color::WHITE.with_alpha(128)));
        let once = canvas.pixel(1, 1);
        assert!((i16::from(once.alpha) - 128).abs() <= 2, "got {once:?}");
        // The colour is preserved rather than being dragged toward black.
        assert_eq!(once.red, 255);
        canvas.fill_rect(canvas.bounds(), &Paint::solid(Color::WHITE.with_alpha(128)));
        assert!(canvas.pixel(1, 1).alpha > once.alpha);
    }

    #[test]
    fn drawing_outside_the_canvas_is_clipped_not_panicking() {
        let mut canvas = Canvas::new(4, 4);
        canvas.clear(Color::BLACK);
        canvas.fill_rect(
            Rect::new(-100.0, -100.0, -50.0, -50.0),
            &Paint::solid(Color::WHITE),
        );
        canvas.fill_rect(
            Rect::new(500.0, 500.0, 900.0, 900.0),
            &Paint::solid(Color::WHITE),
        );
        canvas.fill_rect(
            Rect::new(-10.0, -10.0, 2.0, 2.0),
            &Paint::solid(Color::WHITE),
        );
        assert_eq!(canvas.pixel(0, 0), Color::WHITE);
        assert_eq!(canvas.pixel(3, 3), Color::BLACK);
    }

    #[test]
    fn a_ring_leaves_its_centre_untouched() {
        let mut canvas = Canvas::new(64, 64);
        canvas.clear(Color::BLACK);
        canvas.fill_path(
            &Path::ring(point(32.0, 32.0), 24.0, 6.0),
            &Paint::solid(Color::WHITE),
        );
        assert_eq!(canvas.pixel(32, 32), Color::BLACK);
        assert_eq!(canvas.pixel(32, 10), Color::WHITE);
    }

    #[test]
    fn opacity_composites_against_the_backdrop() {
        let mut canvas = Canvas::new(4, 4);
        canvas.clear(Color::BLACK);
        canvas.fill_rect(canvas.bounds(), &Paint::solid(Color::WHITE.with_alpha(128)));
        let blended = canvas.pixel(1, 1);
        assert!((i16::from(blended.red) - 128).abs() <= 2);
        assert_eq!(blended.alpha, 255);
    }

    #[test]
    fn compositing_a_layer_respects_an_explicit_destination_clip() {
        let mut source = Canvas::new(4, 4);
        source.clear(Color::WHITE);
        let mut target = Canvas::new(6, 6);
        target.clear(Color::BLACK);

        target.draw_canvas_clipped(&source, 1, 1, 1.0, Rect::new(2.0, 2.0, 4.0, 4.0));

        assert_eq!(target.pixel(1, 1), Color::BLACK);
        assert_eq!(target.pixel(2, 2), Color::WHITE);
        assert_eq!(target.pixel(3, 3), Color::WHITE);
        assert_eq!(target.pixel(4, 4), Color::BLACK);
    }

    #[test]
    fn a_gradient_varies_across_the_surface() {
        let mut canvas = Canvas::new(64, 4);
        canvas.fill_rect(
            canvas.bounds(),
            &Paint::horizontal(0.0, 64.0, Color::BLACK, Color::WHITE),
        );
        assert!(canvas.pixel(1, 1).red < canvas.pixel(32, 1).red);
        assert!(canvas.pixel(32, 1).red < canvas.pixel(62, 1).red);
    }

    #[test]
    fn blurring_spreads_coverage_beyond_the_original_edge() {
        let path = Path::rect(Rect::new(20.0, 20.0, 40.0, 40.0));
        let mut mask = Mask::for_path(&path, 12.0);
        let outside_x = (18 - (20 - 12)) as u32;
        assert_eq!(mask.coverage_at(outside_x, 20), 0.0);
        mask.blur(6.0);
        assert!(mask.coverage_at(outside_x, 20) > 0.0);
    }

    #[test]
    fn blurring_conserves_the_bulk_of_its_energy() {
        let path = Path::rect(Rect::new(30.0, 30.0, 60.0, 60.0));
        let mut mask = Mask::for_path(&path, 24.0);
        let before: f32 = (0..mask.height())
            .flat_map(|y| (0..mask.width()).map(move |x| (x, y)))
            .map(|(x, y)| mask.coverage_at(x, y))
            .sum();
        mask.blur(8.0);
        let after: f32 = (0..mask.height())
            .flat_map(|y| (0..mask.width()).map(move |x| (x, y)))
            .map(|(x, y)| mask.coverage_at(x, y))
            .sum();
        assert!(
            (after / before - 1.0).abs() < 0.05,
            "blur lost energy: {before} -> {after}"
        );
    }

    #[test]
    fn a_glow_darkens_with_distance_from_the_shape() {
        let mut canvas = Canvas::new(120, 120);
        canvas.clear(Color::BLACK);
        let path = Path::circle(point(60.0, 60.0), 20.0);
        canvas.draw_glow(&path, 14.0, 2, &Paint::solid(Color::WHITE.with_alpha(160)));
        let near = canvas.pixel(60, 34).red;
        let far = canvas.pixel(60, 20).red;
        assert!(near > far, "glow should fall off: near {near}, far {far}");
    }

    #[test]
    fn sampling_is_clamped_to_the_canvas() {
        let mut canvas = Canvas::new(4, 4);
        canvas.clear(Color::rgb(10, 20, 30));
        assert_eq!(canvas.sample(point(-500.0, -500.0)), Color::rgb(10, 20, 30));
        assert_eq!(canvas.sample(point(500.0, 500.0)), Color::rgb(10, 20, 30));
    }

    #[test]
    fn a_stroke_covers_the_edge_and_spares_the_interior() {
        let mut canvas = Canvas::new(64, 64);
        canvas.clear(Color::BLACK);
        canvas.stroke_rounded_rect(
            Rect::new(10.0, 10.0, 54.0, 54.0),
            8.0,
            4.0,
            &Paint::solid(Color::WHITE),
        );
        assert_eq!(canvas.pixel(32, 32), Color::BLACK);
        assert!(canvas.pixel(32, 10).red > 200);
    }
}
