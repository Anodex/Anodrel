//! Raster images: sampling, filtered resizing, and compositing.
//!
//! The canvas draws vectors, but the brand mark is an authored asset rather
//! than a reconstruction of one. This module is the seam that lets a raster
//! participate in the same pipeline — scaled, faded, and composited alongside
//! everything the rasterizer produces.
//!
//! Filtering is done in **premultiplied** space throughout. Averaging straight
//! alpha would let a transparent pixel's colour bleed into its neighbours,
//! which shows up as a light fringe around every edge of a cut-out mark.

use crate::color::Color;
use crate::geometry::Rect;
use crate::surface::Canvas;

/// A straight-alpha RGBA image.
#[derive(Clone)]
pub struct Image {
    width: u32,
    height: u32,
    pixels: Vec<Color>,
}

impl Image {
    /// Builds an image from straight-alpha pixels in row-major order.
    ///
    /// Returns `None` unless `pixels` is exactly `width * height` long.
    #[must_use]
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<Color>) -> Option<Self> {
        if pixels.len() != (width as usize) * (height as usize) {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    /// Builds an image from packed `B, G, R, A` bytes.
    ///
    /// This is the layout Windows bitmaps and the committed brand asset use, so
    /// an embedded asset needs no conversion pass.
    #[must_use]
    pub fn from_bgra_bytes(width: u32, height: u32, bytes: &[u8]) -> Option<Self> {
        let expected = (width as usize) * (height as usize) * 4;
        if bytes.len() != expected {
            return None;
        }
        let pixels = bytes
            .chunks_exact(4)
            .map(|pixel| Color::rgba(pixel[2], pixel[1], pixel[0], pixel[3]))
            .collect();
        Self::from_pixels(width, height, pixels)
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

    /// Returns `true` when the image holds no pixels.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Returns a pixel, or transparent when out of bounds.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Color {
        if x >= self.width || y >= self.height {
            return Color::TRANSPARENT;
        }
        self.pixels[(y as usize) * (self.width as usize) + (x as usize)]
    }

    /// Returns the tight bounding box of pixels above `threshold` alpha.
    ///
    /// Used to crop an authored asset to its artwork, so a raster and the
    /// equivalent geometry place identically inside the same bounds.
    #[must_use]
    pub fn opaque_bounds(&self, threshold: u8) -> Option<(u32, u32, u32, u32)> {
        let (mut left, mut top) = (self.width, self.height);
        let (mut right, mut bottom) = (0_u32, 0_u32);
        let mut found = false;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.pixel(x, y).alpha > threshold {
                    found = true;
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                }
            }
        }
        found.then_some((left, top, right + 1, bottom + 1))
    }

    /// Returns a sub-rectangle as a new image.
    #[must_use]
    pub fn cropped(&self, left: u32, top: u32, width: u32, height: u32) -> Self {
        let mut pixels = Vec::with_capacity((width as usize) * (height as usize));
        for y in 0..height {
            for x in 0..width {
                pixels.push(self.pixel(left + x, top + y));
            }
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Returns the image resampled to a new size.
    ///
    /// Each destination pixel averages its whole source footprint, so reducing
    /// an image does not alias the way point sampling would. Enlarging falls
    /// out of the same formula as bilinear interpolation, because a footprint
    /// narrower than one pixel covers at most two samples per axis.
    #[must_use]
    pub fn resized(&self, width: u32, height: u32) -> Self {
        if width == 0 || height == 0 || self.is_empty() {
            return Self {
                width,
                height,
                pixels: vec![Color::TRANSPARENT; (width as usize) * (height as usize)],
            };
        }
        if width == self.width && height == self.height {
            return self.clone();
        }

        let scale_x = self.width as f32 / width as f32;
        let scale_y = self.height as f32 / height as f32;
        let mut pixels = Vec::with_capacity((width as usize) * (height as usize));

        for y in 0..height {
            let source_top = y as f32 * scale_y;
            let source_bottom = source_top + scale_y;
            let first_row = source_top.floor() as u32;
            let last_row = ((source_bottom.ceil() as u32).max(first_row + 1)).min(self.height);
            for x in 0..width {
                let source_left = x as f32 * scale_x;
                let source_right = source_left + scale_x;
                let first_column = source_left.floor() as u32;
                let last_column =
                    ((source_right.ceil() as u32).max(first_column + 1)).min(self.width);

                let (mut red, mut green, mut blue, mut alpha, mut weight_total) =
                    (0.0, 0.0, 0.0, 0.0, 0.0);
                for row in first_row..last_row {
                    let row_weight = span_overlap(source_top, source_bottom, row);
                    if row_weight <= 0.0 {
                        continue;
                    }
                    for column in first_column..last_column {
                        let weight = row_weight * span_overlap(source_left, source_right, column);
                        if weight <= 0.0 {
                            continue;
                        }
                        let sample = self.pixel(column, row);
                        // Premultiply before averaging: a transparent pixel must
                        // contribute no colour, only its (zero) weight.
                        let sample_alpha = f32::from(sample.alpha) / 255.0;
                        red += f32::from(sample.red) * sample_alpha * weight;
                        green += f32::from(sample.green) * sample_alpha * weight;
                        blue += f32::from(sample.blue) * sample_alpha * weight;
                        alpha += sample_alpha * weight;
                        weight_total += weight;
                    }
                }

                pixels.push(if weight_total <= 0.0 || alpha <= 0.0 {
                    Color::TRANSPARENT
                } else {
                    // Undo the premultiply so the result is straight alpha again.
                    Color::rgba(
                        (red / alpha).round().clamp(0.0, 255.0) as u8,
                        (green / alpha).round().clamp(0.0, 255.0) as u8,
                        (blue / alpha).round().clamp(0.0, 255.0) as u8,
                        ((alpha / weight_total) * 255.0).round().clamp(0.0, 255.0) as u8,
                    )
                });
            }
        }

        Self {
            width,
            height,
            pixels,
        }
    }

    /// Samples the image at normalised coordinates with bilinear filtering.
    ///
    /// Coordinates outside `0.0..=1.0` clamp to the edge.
    #[must_use]
    pub fn sample_bilinear(&self, u: f32, v: f32) -> Color {
        if self.is_empty() {
            return Color::TRANSPARENT;
        }
        let x = (u * self.width as f32 - 0.5).clamp(0.0, self.width as f32 - 1.0);
        let y = (v * self.height as f32 - 0.5).clamp(0.0, self.height as f32 - 1.0);
        let (left, top) = (x.floor(), y.floor());
        let (fraction_x, fraction_y) = (x - left, y - top);
        let right = (left as u32 + 1).min(self.width - 1);
        let bottom = (top as u32 + 1).min(self.height - 1);

        let corners = [
            (
                self.pixel(left as u32, top as u32),
                (1.0 - fraction_x) * (1.0 - fraction_y),
            ),
            (
                self.pixel(right, top as u32),
                fraction_x * (1.0 - fraction_y),
            ),
            (
                self.pixel(left as u32, bottom),
                (1.0 - fraction_x) * fraction_y,
            ),
            (self.pixel(right, bottom), fraction_x * fraction_y),
        ];
        let (mut red, mut green, mut blue, mut alpha) = (0.0, 0.0, 0.0, 0.0);
        for (sample, weight) in corners {
            let sample_alpha = f32::from(sample.alpha) / 255.0;
            red += f32::from(sample.red) * sample_alpha * weight;
            green += f32::from(sample.green) * sample_alpha * weight;
            blue += f32::from(sample.blue) * sample_alpha * weight;
            alpha += sample_alpha * weight;
        }
        if alpha <= 0.0 {
            return Color::TRANSPARENT;
        }
        Color::rgba(
            (red / alpha).round().clamp(0.0, 255.0) as u8,
            (green / alpha).round().clamp(0.0, 255.0) as u8,
            (blue / alpha).round().clamp(0.0, 255.0) as u8,
            (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        )
    }

    /// Returns the image as packed `B, G, R, A` bytes.
    #[must_use]
    pub fn to_bgra_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in &self.pixels {
            bytes.extend_from_slice(&[pixel.blue, pixel.green, pixel.red, pixel.alpha]);
        }
        bytes
    }
}

/// Returns how much of pixel `index` lies inside the span `from..to`.
fn span_overlap(from: f32, to: f32, index: u32) -> f32 {
    let low = (index as f32).max(from);
    let high = ((index + 1) as f32).min(to);
    (high - low).max(0.0)
}

impl Canvas {
    /// Composites an image into a destination rectangle.
    ///
    /// The image is bilinearly sampled, so `dest` may be any size. For a large
    /// reduction, resize once with [`Image::resized`] and draw from that — a
    /// bilinear sample cannot represent a footprint several pixels wide, and
    /// resampling every frame would be wasteful anyway.
    ///
    /// `opacity` scales the whole image and is clamped to `0.0..=1.0`.
    pub fn draw_image(&mut self, image: &Image, dest: Rect, opacity: f32) {
        let opacity = opacity.clamp(0.0, 1.0);
        if image.is_empty() || dest.is_empty() || opacity <= 0.0 {
            return;
        }
        let clip = dest.intersect(self.bounds());
        if clip.is_empty() {
            return;
        }
        let (width, height) = (dest.width(), dest.height());
        let x_start = clip.left.floor().max(0.0) as u32;
        let x_end = clip.right.ceil().min(self.width() as f32) as u32;
        let y_start = clip.top.floor().max(0.0) as u32;
        let y_end = clip.bottom.ceil().min(self.height() as f32) as u32;

        for y in y_start..y_end {
            let v = (y as f32 + 0.5 - dest.top) / height;
            for x in x_start..x_end {
                let u = (x as f32 + 0.5 - dest.left) / width;
                let sample = image.sample_bilinear(u, v);
                if sample.alpha == 0 {
                    continue;
                }
                self.blend_pixel(x, y, sample.scale_alpha(opacity));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Image;
    use crate::color::Color;
    use crate::geometry::Rect;
    use crate::surface::Canvas;

    fn checker(size: u32) -> Image {
        let pixels = (0..size * size)
            .map(|index| {
                let (x, y) = (index % size, index / size);
                if (x + y) % 2 == 0 {
                    Color::WHITE
                } else {
                    Color::BLACK
                }
            })
            .collect();
        Image::from_pixels(size, size, pixels).expect("checker builds")
    }

    #[test]
    fn bgra_bytes_round_trip() {
        let source = checker(4);
        let bytes = source.to_bgra_bytes();
        let restored = Image::from_bgra_bytes(4, 4, &bytes).expect("restores");
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(restored.pixel(x, y), source.pixel(x, y));
            }
        }
    }

    #[test]
    fn a_wrongly_sized_byte_buffer_is_rejected() {
        assert!(Image::from_bgra_bytes(4, 4, &[0; 8]).is_none());
        assert!(Image::from_pixels(2, 2, vec![Color::WHITE; 3]).is_none());
    }

    #[test]
    fn reducing_a_checkerboard_averages_rather_than_aliasing() {
        // Point sampling would return pure black or pure white; a box filter
        // must land near the mean.
        let reduced = checker(64).resized(1, 1);
        let grey = reduced.pixel(0, 0);
        assert!(
            (i16::from(grey.red) - 128).abs() <= 4,
            "expected the mean, got {grey:?}"
        );
    }

    #[test]
    fn resizing_to_the_same_size_is_lossless() {
        let source = checker(8);
        let same = source.resized(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(same.pixel(x, y), source.pixel(x, y));
            }
        }
    }

    #[test]
    fn filtering_does_not_bleed_colour_out_of_transparent_pixels() {
        // A red pixel beside a fully transparent one. Averaging straight alpha
        // would drag the result toward black; premultiplied averaging must keep
        // the hue and only reduce the alpha.
        let pixels = vec![
            Color::rgba(255, 0, 0, 255),
            Color::rgba(0, 0, 0, 0),
            Color::rgba(255, 0, 0, 255),
            Color::rgba(0, 0, 0, 0),
        ];
        let image = Image::from_pixels(2, 2, pixels).expect("builds");
        let reduced = image.resized(1, 1);
        let blended = reduced.pixel(0, 0);
        assert_eq!(blended.red, 255, "hue must survive: {blended:?}");
        assert_eq!(blended.green, 0);
        assert!(
            (i16::from(blended.alpha) - 128).abs() <= 2,
            "alpha should halve: {blended:?}"
        );
    }

    #[test]
    fn opaque_bounds_finds_the_artwork_inside_padding() {
        let mut pixels = vec![Color::TRANSPARENT; 100];
        pixels[3 * 10 + 2] = Color::WHITE;
        pixels[6 * 10 + 7] = Color::WHITE;
        let image = Image::from_pixels(10, 10, pixels).expect("builds");
        assert_eq!(image.opaque_bounds(0), Some((2, 3, 8, 7)));
    }

    #[test]
    fn opaque_bounds_of_an_empty_image_is_none() {
        let image = Image::from_pixels(4, 4, vec![Color::TRANSPARENT; 16]).expect("builds");
        assert!(image.opaque_bounds(0).is_none());
    }

    #[test]
    fn cropping_extracts_the_requested_region() {
        let source = checker(8);
        let cropped = source.cropped(2, 3, 4, 4);
        assert_eq!(cropped.width(), 4);
        assert_eq!(cropped.pixel(0, 0), source.pixel(2, 3));
        assert_eq!(cropped.pixel(3, 3), source.pixel(5, 6));
    }

    #[test]
    fn drawing_an_image_fills_its_destination() {
        let image = Image::from_pixels(2, 2, vec![Color::rgb(200, 60, 240); 4]).expect("builds");
        let mut canvas = Canvas::new(32, 32);
        canvas.clear(Color::BLACK);
        canvas.draw_image(&image, Rect::new(8.0, 8.0, 24.0, 24.0), 1.0);
        assert_eq!(canvas.pixel(16, 16), Color::rgb(200, 60, 240));
        assert_eq!(canvas.pixel(4, 4), Color::BLACK, "outside the destination");
    }

    #[test]
    fn opacity_fades_a_drawn_image() {
        let image = Image::from_pixels(1, 1, vec![Color::WHITE]).expect("builds");
        let mut canvas = Canvas::new(8, 8);
        canvas.clear(Color::BLACK);
        canvas.draw_image(&image, canvas.bounds(), 0.5);
        let faded = canvas.pixel(4, 4);
        assert!((i16::from(faded.red) - 128).abs() <= 2, "got {faded:?}");
    }

    #[test]
    fn a_transparent_image_leaves_the_canvas_alone() {
        let image = Image::from_pixels(2, 2, vec![Color::TRANSPARENT; 4]).expect("builds");
        let mut canvas = Canvas::new(8, 8);
        canvas.clear(Color::BLACK);
        canvas.draw_image(&image, canvas.bounds(), 1.0);
        assert_eq!(canvas.pixel(4, 4), Color::BLACK);
    }

    #[test]
    fn drawing_outside_the_canvas_is_clipped() {
        let image = checker(4);
        let mut canvas = Canvas::new(8, 8);
        canvas.clear(Color::BLACK);
        canvas.draw_image(&image, Rect::new(-100.0, -100.0, -50.0, -50.0), 1.0);
        canvas.draw_image(&image, Rect::new(400.0, 400.0, 500.0, 500.0), 1.0);
        assert_eq!(canvas.pixel(4, 4), Color::BLACK);
    }
}
