//! Coverage rasterization and pixel compositing primitives.

use crate::color::Color;
use crate::geometry::Rect;
use crate::path::Path;

/// Vertical samples taken per pixel row when computing edge coverage.
///
/// Horizontal coverage is exact — spans carry fractional endpoints — so only
/// the vertical axis is sampled. Eight rows put the residual error below what
/// an 8-bit channel can represent on all but the shallowest edges.
const SUBSAMPLES: usize = 8;

/// Coverage below this is dropped; it cannot change an 8-bit channel.
pub(crate) const COVERAGE_EPSILON: f32 = 1.0 / 512.0;

/// Walks a path and reports per-pixel coverage in `0.0..=1.0`.
///
/// Each pixel row is sampled at [`SUBSAMPLES`] sub-scanlines. For every
/// sub-scanline the edge crossings are sorted and paired under the non-zero
/// winding rule, then the resulting spans are accumulated with exact fractional
/// endpoints. Cost is proportional to covered area rather than to canvas size.
pub(crate) fn rasterize(path: &Path, width: u32, height: u32, mut emit: impl FnMut(u32, u32, f32)) {
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
pub(crate) fn blend_into(
    pixels: &mut [u32],
    width: u32,
    x: u32,
    y: u32,
    color: Color,
    coverage: f32,
) {
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
