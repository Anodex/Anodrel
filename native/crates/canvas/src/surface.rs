//! The coverage rasterizer, the pixel buffer, and the blurred coverage mask.

use crate::color::Color;
use crate::geometry::{Point, Rect};
use crate::mask::Mask;
use crate::paint::Paint;
use crate::path::Path;
use crate::raster::{COVERAGE_EPSILON, blend_into, rasterize};

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

#[cfg(test)]
mod tests;
