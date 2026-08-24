//! Reusable anti-aliased coverage masks and their blur implementation.

use crate::path::Path;
use crate::raster::rasterize;

/// An anti-aliased coverage buffer that can be blurred and reused.
///
/// A mask is positioned by its origin in canvas space and sized to the geometry
/// it holds, so a glow does not pay for the whole surface.
#[derive(Clone)]
pub struct Mask {
    /// Horizontal position in the target canvas.
    pub(crate) origin_x: i32,
    /// Vertical position in the target canvas.
    pub(crate) origin_y: i32,
    /// Width of the coverage buffer.
    pub(crate) width: u32,
    /// Height of the coverage buffer.
    pub(crate) height: u32,
    /// Row-major fractional coverage values.
    pub(crate) coverage: Vec<f32>,
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
        let mut moved = self.clone();
        moved.reposition(origin_x, origin_y);
        moved
    }

    /// Moves this mask to a new origin in canvas space.
    ///
    /// A mask's coverage does not depend on where it is placed, so a retained
    /// mask can be composited at a new position without being rebuilt or
    /// copied. That matters for a blurred mask: its coverage buffer is one
    /// `f32` per pixel, so a mark-sized glow is around half a megabyte that
    /// [`positioned`](Self::positioned) would duplicate on every frame.
    pub const fn reposition(&mut self, origin_x: i32, origin_y: i32) {
        self.origin_x = origin_x;
        self.origin_y = origin_y;
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

    /// Returns the box radius [`blur`](Self::blur) uses for a blur radius.
    ///
    /// The blur quantizes the radius it is given, so two radii that round to
    /// the same box radius produce identical output. A caller retaining a
    /// blurred mask can key on this rather than on the radius it asked for.
    #[must_use]
    pub fn blur_box_radius(radius: f32) -> usize {
        ((radius / 3.0).round() as usize).max(1)
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
        let box_radius = Self::blur_box_radius(radius);
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
