//! Fixed drawing stages for the owned software rasterizer.
//!
//! Anodrel composes every first-party surface in software, so a frame's cost is
//! the sum of a handful of rasterizer operations. Timing a whole surface tells
//! you a frame got slower; timing these tells you which stage did.
//!
//! The stages are the ones that actually dominate a Startup Lab frame, measured
//! by instrumenting `startup_lab::draw` and recorded in `docs/PERFORMANCE.md`.
//! Nothing here opens a window or presents anything: this is the rasterizer's
//! own cost, which is deliberately *less* than the cost of a frame reaching the
//! screen. The report's scope string says so.
//!
//! Sizes are fixed constants rather than options. A measurement whose workload
//! can be varied at the command line is not comparable with one someone else
//! recorded.

use std::{hint::black_box, time::Instant};

use anodrel_canvas::{Canvas, Color, Image, Mask, Paint, Path, Rect, Stop, point};

use crate::report::{Dimension, LatencyMeasurement};

use super::measurement;

/// Warmup passes per stage, discarded before measuring.
///
/// Fewer than the transport workload's: a drawing stage touches a large buffer,
/// so its caches settle in far fewer passes and the warmup is pure wall time.
const WARMUP_PASSES: usize = 20;

/// The surface size every stage is measured against.
///
/// The Startup Lab's authored size, so a stage cost here can be compared with
/// the frame budget in `docs/PERFORMANCE.md` directly.
const SURFACE_WIDTH: u32 = 1_240;
const SURFACE_HEIGHT: u32 = 900;

/// Edge of the square region the mask stages cover.
///
/// The mark's glow mask is 366 pixels square on the Startup Lab, so this is the
/// real shape of the most expensive mask the platform builds.
const MASK_EDGE: u32 = 366;

/// Blur radius applied to the mask stages, matching the hero mark's glow.
const BLUR_RADIUS: f32 = 48.4;

/// Edge of the source image the scaling stage composites.
const IMAGE_EDGE: u32 = 256;

/// Region the image stage draws into, matching the hero mark's bounds.
const IMAGE_TARGET_EDGE: f32 = 220.0;

/// One measured drawing stage: what it is called, what it covers, what it does.
struct Stage {
    name: &'static str,
    pixels: u64,
    draw: StageFn,
}

/// Every stage takes the same three fixtures so one timing loop serves them all.
type StageFn = fn(&mut Canvas, &Image, &Mask);

/// Measures every stage, in report order.
pub(super) fn measurements(iterations: usize) -> Result<Vec<LatencyMeasurement>, String> {
    let stages = [
        Stage {
            name: "surface-clear",
            pixels: surface_pixels(),
            draw: clear_stage,
        },
        Stage {
            name: "gradient-panel",
            pixels: panel_pixels(),
            draw: panel_stage,
        },
        Stage {
            name: "mask-blur",
            pixels: mask_pixels(),
            draw: blur_stage,
        },
        Stage {
            name: "mask-fill-gradient",
            pixels: mask_pixels(),
            draw: mask_fill_stage,
        },
        Stage {
            name: "image-scale",
            pixels: image_target_pixels(),
            draw: image_stage,
        },
    ];

    let mut canvas = Canvas::new(SURFACE_WIDTH, SURFACE_HEIGHT);
    let image = source_image();
    let mask = coverage_mask();

    stages
        .into_iter()
        .map(|stage| {
            let samples = samples_for(&mut canvas, &image, &mask, stage.draw, iterations);
            measurement(
                Dimension::Stage {
                    name: stage.name,
                    pixels: stage.pixels,
                },
                samples,
            )
        })
        .collect()
}

fn samples_for(
    canvas: &mut Canvas,
    image: &Image,
    mask: &Mask,
    stage: StageFn,
    iterations: usize,
) -> Vec<u128> {
    for _ in 0..WARMUP_PASSES {
        stage(canvas, image, mask);
    }
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        stage(black_box(canvas), black_box(image), black_box(mask));
        samples.push(started.elapsed().as_nanos());
    }
    samples
}

/// Filling every pixel with a flat colour: the floor every frame pays.
fn clear_stage(canvas: &mut Canvas, _image: &Image, _mask: &Mask) {
    canvas.clear(Color::hex(0x0A0E1A));
}

/// One rounded panel filled through a linear gradient, as a status card is.
fn panel_stage(canvas: &mut Canvas, _image: &Image, _mask: &Mask) {
    canvas.fill_rounded_rect(panel_bounds(), 14.0, &panel_paint());
}

/// Blurring a coverage mask, the stage behind every glow and shadow.
///
/// The mask is cloned per pass because blurring consumes it; the clone is a
/// buffer copy and is included in the sample, which is honest — a caller that
/// blurs has to get an unblurred mask from somewhere.
fn blur_stage(_canvas: &mut Canvas, _image: &Image, mask: &Mask) {
    let mut blurred = mask.clone();
    blurred.blur(BLUR_RADIUS);
    black_box(blurred.coverage_at(0, 0));
}

/// Compositing a blurred mask through a gradient.
///
/// Identified as the largest single cost in a Startup Lab reveal frame once the
/// glow mask itself was retained; see Decision 0064.
fn mask_fill_stage(canvas: &mut Canvas, _image: &Image, mask: &Mask) {
    canvas.fill_mask(mask, &panel_paint());
}

/// Compositing a scaled image, as the mark's artwork is drawn.
fn image_stage(canvas: &mut Canvas, image: &Image, _mask: &Mask) {
    canvas.draw_image(image, image_bounds(), 1.0);
}

fn panel_bounds() -> Rect {
    Rect::new(40.0, 40.0, 40.0 + 280.0, 40.0 + 150.0)
}

fn panel_paint() -> Paint {
    Paint::linear(
        point(40.0, 0.0),
        point(320.0, 0.0),
        vec![
            Stop::new(0.0, Color::hex(0xA855F7).with_alpha(190)),
            Stop::new(0.5, Color::hex(0x7C3AED).with_alpha(160)),
            Stop::new(1.0, Color::hex(0x3B82F6).with_alpha(190)),
        ],
    )
}

fn image_bounds() -> Rect {
    Rect::new(
        400.0,
        300.0,
        400.0 + IMAGE_TARGET_EDGE,
        300.0 + IMAGE_TARGET_EDGE,
    )
}

/// A deterministic source image, so the stage measures scaling and not decoding.
///
/// Synthesized rather than taken from the brand crate: this tool measures the
/// rasterizer, and depending on the artwork would tie a performance number to a
/// design asset that is free to change.
fn source_image() -> Image {
    let side = IMAGE_EDGE as usize;
    let mut bytes = Vec::with_capacity(side * side * 4);
    for y in 0..side {
        for x in 0..side {
            // A gradient with a soft alpha ramp, so both the colour and the
            // alpha paths are exercised rather than a constant fill.
            let u = (x * 255 / side.max(1)) as u8;
            let v = (y * 255 / side.max(1)) as u8;
            bytes.extend_from_slice(&[v, u.wrapping_add(64), 255 - u, u | 0x40]);
        }
    }
    Image::from_bgra_bytes(IMAGE_EDGE, IMAGE_EDGE, &bytes).expect("the synthesized image is square")
}

/// A blurred coverage mask of the size the hero mark's glow really uses.
fn coverage_mask() -> Mask {
    let inset = f32::from(u16::try_from(MASK_EDGE).unwrap_or(u16::MAX)) * 0.2;
    let edge = MASK_EDGE as f32;
    let mut mask = Mask::new(0, 0, MASK_EDGE, MASK_EDGE);
    mask.fill_path(&Path::rounded_rect(
        Rect::new(inset, inset, edge - inset, edge - inset),
        24.0,
    ));
    mask.blur(BLUR_RADIUS);
    mask
}

const fn surface_pixels() -> u64 {
    SURFACE_WIDTH as u64 * SURFACE_HEIGHT as u64
}

const fn mask_pixels() -> u64 {
    MASK_EDGE as u64 * MASK_EDGE as u64
}

fn panel_pixels() -> u64 {
    let bounds = panel_bounds();
    (bounds.width() * bounds.height()) as u64
}

const fn image_target_pixels() -> u64 {
    (IMAGE_TARGET_EDGE as u64) * (IMAGE_TARGET_EDGE as u64)
}

#[cfg(test)]
mod tests {
    use super::{
        MASK_EDGE, coverage_mask, image_target_pixels, mask_pixels, measurements, source_image,
        surface_pixels,
    };

    #[test]
    fn every_stage_reports_a_distinct_name_and_a_nonzero_pixel_count() {
        let measured = measurements(3).expect("the renderer workload runs");
        assert_eq!(measured.len(), 5);

        let mut names = Vec::new();
        for measurement in &measured {
            assert_eq!(measurement.samples, 3);
            let crate::report::Dimension::Stage { name, pixels } = measurement.dimension else {
                panic!("a renderer stage reported a payload size");
            };
            assert!(pixels > 0, "{name} reports no pixels");
            names.push(name);
        }
        let unique = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), unique, "two stages share a name");
    }

    #[test]
    fn the_fixtures_match_the_sizes_the_report_publishes() {
        // The pixel counts are what a reader divides by to get a per-pixel
        // cost, so they have to describe the work that actually happened.
        assert_eq!(surface_pixels(), 1_240 * 900);
        assert_eq!(mask_pixels(), u64::from(MASK_EDGE) * u64::from(MASK_EDGE));
        assert_eq!(image_target_pixels(), 220 * 220);

        let mask = coverage_mask();
        assert_eq!(mask.width(), MASK_EDGE);
        assert_eq!(mask.height(), MASK_EDGE);

        let image = source_image();
        assert_eq!(image.width(), 256);
    }
}
