//! Fixed local evidence for the owned Windows text composition path.
//!
//! This is intentionally not the Windows text painter. It accepts no caller
//! value and changes no host surface; see Decisions 0212 and 0213 for its
//! fixed boundary.

use std::{io, time::Instant};

use anodrel_canvas::{Canvas, Color, Paint, point};
use anodrel_font::FontFace;
use anodrel_glyph::GlyphMaskCache;
use anodrel_text::TextRun;

use super::{owned_text, selected_font, text};

const PROBE_TEXT: &str = "ANODREL";
const PROBE_EM_PIXELS: f32 = 32.0;
const PROBE_WEIGHT: i32 = 400;
const PROBE_WIDTH: u32 = 512;
const PROBE_HEIGHT: u32 = 144;
const PROBE_LEFT: f32 = 24.0;
const PROBE_BASELINES: [f32; 2] = [48.0, 112.0];
#[cfg(test)]
const MAX_FIXED_ADVANCE_DRIFT_MILLI_PIXELS: i128 = 1_000;

/// Private closed outcomes from the fixed owned-text composition probe.
#[derive(Debug)]
pub(super) enum OwnedTextProbeError {
    /// Windows did not provide the fixed selected face under its source bounds.
    Source,
    /// The selected face failed the owned parser's validation.
    InvalidFace,
    /// The fixed source value could not form one owned text run.
    Run,
    /// One owned glyph could not provide bounded cached coverage.
    Glyph,
    /// The selected GDI route could not measure the fixed diagnostic value.
    GdiMetric,
    /// The fixed run could not convert its bounded advance into report units.
    Advance,
    /// The exact same glyph set did not remain cached after the second row.
    Cache,
    /// The fixed composition did not leave any visible coverage in its canvas.
    Canvas,
}

/// Private result facts from one fixed owned-text composition.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct OwnedTextProbe {
    pub(super) canvas: Canvas,
    pub(super) glyph_count: usize,
    pub(super) retained_mask_count: usize,
    pub(super) retained_pixel_count: usize,
}

/// Closed facts emitted by the fixed local owned-text report.
struct OwnedTextReport {
    source_bytes: usize,
    glyph_count: usize,
    gdi_advance_pixels: u64,
    owned_advance_milli_pixels: i64,
    retained_mask_count: usize,
    retained_pixel_count: usize,
    first_row_microseconds: u64,
    reused_row_microseconds: u64,
}

/// Composes the fixed probe through every current owned text layer exactly twice.
///
/// No host route invokes this during normal presentation. The second row has an
/// integer-only baseline change, which must reuse the first row's cached masks.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn compose() -> Result<OwnedTextProbe, OwnedTextProbeError> {
    with_owned_run(|_, face, run, pixels_per_design_unit| {
        let mut cache = GlyphMaskCache::new(face);
        let mut canvas = empty_canvas();
        let paint = Paint::solid(Color::WHITE);
        for baseline_y in PROBE_BASELINES {
            draw_row(
                &mut canvas,
                &mut cache,
                run,
                pixels_per_design_unit,
                baseline_y,
                &paint,
            )?;
        }
        Ok(OwnedTextProbe {
            canvas,
            glyph_count: run.glyphs().len(),
            retained_mask_count: cache.retained_mask_count(),
            retained_pixel_count: cache.retained_pixel_count(),
        })
    })
}

/// Prints one fixed local report without opening a window or accepting input.
pub(crate) fn run_report() -> io::Result<()> {
    let report = inspect().map_err(|_| io::Error::other("owned-text report did not complete"))?;
    println!("{}", format_report(&report));
    Ok(())
}

/// Encodes the closed report schema without accepting diagnostic input.
fn format_report(report: &OwnedTextReport) -> String {
    format!(
        concat!(
            "{{\"benchmark\":\"anodrel.host.owned-text.v1\",",
            "\"sourceBytes\":{},\"glyphCount\":{},",
            "\"gdiAdvancePixels\":{},\"ownedAdvanceMilliPixels\":{},",
            "\"retainedMaskCount\":{},\"retainedPixelCount\":{},",
            "\"firstRowMicroseconds\":{},\"reusedRowMicroseconds\":{},",
            "\"scope\":\"fixed selected Windows face; local diagnostic only; no visible painter, quality, or release-performance claim\"}}"
        ),
        report.source_bytes,
        report.glyph_count,
        report.gdi_advance_pixels,
        report.owned_advance_milli_pixels,
        report.retained_mask_count,
        report.retained_pixel_count,
        report.first_row_microseconds,
        report.reused_row_microseconds,
    )
}

/// Collects the fixed report's facts before anything reaches standard output.
fn inspect() -> Result<OwnedTextReport, OwnedTextProbeError> {
    let gdi_advance_pixels = gdi_advance_pixels()?;
    with_owned_run(|source, face, run, pixels_per_design_unit| {
        let mut cache = GlyphMaskCache::new(face);
        let mut canvas = empty_canvas();
        let paint = Paint::solid(Color::WHITE);
        let first_row_microseconds = draw_timed_row(
            &mut canvas,
            &mut cache,
            run,
            pixels_per_design_unit,
            PROBE_BASELINES[0],
            &paint,
        )?;
        let reused_row_microseconds = draw_timed_row(
            &mut canvas,
            &mut cache,
            run,
            pixels_per_design_unit,
            PROBE_BASELINES[1],
            &paint,
        )?;
        let glyph_count = run.glyphs().len();
        if glyph_count == 0 || cache.retained_mask_count() != glyph_count {
            return Err(OwnedTextProbeError::Cache);
        }
        if !canvas_has_ink(&canvas) {
            return Err(OwnedTextProbeError::Canvas);
        }
        Ok(OwnedTextReport {
            source_bytes: source.len(),
            glyph_count,
            gdi_advance_pixels,
            owned_advance_milli_pixels: owned_advance_milli_pixels(run, pixels_per_design_unit)?,
            retained_mask_count: cache.retained_mask_count(),
            retained_pixel_count: cache.retained_pixel_count(),
            first_row_microseconds,
            reused_row_microseconds,
        })
    })
}

/// Builds the fixed selected source, validated face, run, and pixel scale once.
fn with_owned_run<T>(
    work: impl FnOnce(&[u8], &FontFace<'_>, &TextRun, f32) -> Result<T, OwnedTextProbeError>,
) -> Result<T, OwnedTextProbeError> {
    let source = selected_font::selected_face_data(PROBE_EM_PIXELS as i32, PROBE_WEIGHT)
        .ok_or(OwnedTextProbeError::Source)?;
    let face = FontFace::parse(&source).map_err(|_| OwnedTextProbeError::InvalidFace)?;
    let run = TextRun::build(&face, PROBE_TEXT).map_err(|_| OwnedTextProbeError::Run)?;
    let pixels_per_design_unit = owned_text::pixels_per_design_unit(&run, PROBE_EM_PIXELS)
        .ok_or(OwnedTextProbeError::Advance)?;
    work(&source, &face, &run, pixels_per_design_unit)
}

/// Builds the fixed transparent drawing target for a local composition.
fn empty_canvas() -> Canvas {
    let mut canvas = Canvas::new(PROBE_WIDTH, PROBE_HEIGHT);
    canvas.clear(Color::BLACK);
    canvas
}

/// Draws the fixed run at one baseline through the owned cache and compositor.
fn draw_row(
    canvas: &mut Canvas,
    cache: &mut GlyphMaskCache<'_>,
    run: &TextRun,
    pixels_per_design_unit: f32,
    baseline_y: f32,
    paint: &Paint,
) -> Result<(), OwnedTextProbeError> {
    owned_text::draw_row(
        canvas,
        cache,
        run,
        pixels_per_design_unit,
        point(PROBE_LEFT, baseline_y),
        paint,
    )
    .map_err(|_| OwnedTextProbeError::Glyph)
}

/// Times exactly one fixed row's owned cache lookup and composition work.
fn draw_timed_row(
    canvas: &mut Canvas,
    cache: &mut GlyphMaskCache<'_>,
    run: &TextRun,
    pixels_per_design_unit: f32,
    baseline_y: f32,
    paint: &Paint,
) -> Result<u64, OwnedTextProbeError> {
    let started = Instant::now();
    draw_row(
        canvas,
        cache,
        run,
        pixels_per_design_unit,
        baseline_y,
        paint,
    )?;
    Ok(started.elapsed().as_micros().try_into().unwrap_or(u64::MAX))
}

/// Reads the current GDI metric for precisely the fixed report text and size.
fn gdi_advance_pixels() -> Result<u64, OwnedTextProbeError> {
    let width = text::width(&text::TextSpec::new(
        PROBE_TEXT,
        PROBE_EM_PIXELS as i32,
        PROBE_WEIGHT,
    ));
    if !width.is_finite() || width <= 0.0 || width > u64::MAX as f32 {
        return Err(OwnedTextProbeError::GdiMetric);
    }
    Ok(width.round() as u64)
}

/// Converts the owned run's bounded advance to a stable report unit.
fn owned_advance_milli_pixels(
    run: &TextRun,
    pixels_per_design_unit: f32,
) -> Result<i64, OwnedTextProbeError> {
    owned_text::advance_milli_pixels(run, pixels_per_design_unit)
        .ok_or(OwnedTextProbeError::Advance)
}

/// Measures the fixed report's run-width difference without widening its schema.
///
/// GDI reports a whole-pixel width while the owned path reports thousandths of a
/// pixel, so a one-pixel allowance covers that deliberate difference in units.
#[cfg(test)]
fn fixed_advance_drift_milli_pixels(report: &OwnedTextReport) -> i128 {
    (i128::from(report.gdi_advance_pixels) * 1_000 - i128::from(report.owned_advance_milli_pixels))
        .abs()
}

/// Reports whether a composition produced any non-background coverage.
fn canvas_has_ink(canvas: &Canvas) -> bool {
    canvas
        .pixels()
        .iter()
        .any(|pixel| *pixel != Color::BLACK.to_argb())
}

#[cfg(test)]
mod tests {
    use super::{
        Color, MAX_FIXED_ADVANCE_DRIFT_MILLI_PIXELS, OwnedTextReport, PROBE_TEXT, compose,
        fixed_advance_drift_milli_pixels, format_report, inspect,
    };

    #[test]
    fn selected_face_completes_the_owned_text_chain_and_reuses_coverage() {
        let probe = compose().expect("selected face completes the owned text probe");
        assert_eq!(probe.glyph_count, PROBE_TEXT.chars().count());
        assert_eq!(probe.retained_mask_count, probe.glyph_count);
        assert!(probe.retained_pixel_count > 0);

        let black = Color::BLACK.to_argb();
        let row_has_ink = |top, bottom| {
            (top..bottom).any(|y| {
                let width = probe.canvas.width() as usize;
                probe.canvas.pixels()[y * width..(y + 1) * width]
                    .iter()
                    .any(|pixel| *pixel != black)
            })
        };
        assert!(row_has_ink(0, 64));
        assert!(row_has_ink(64, probe.canvas.height() as usize));
    }

    #[test]
    fn fixed_report_captures_the_selected_face_and_exact_cache_reuse() {
        let report = inspect().expect("fixed owned-text report completes");
        assert!(report.source_bytes > 0);
        assert_eq!(report.glyph_count, PROBE_TEXT.chars().count());
        assert!(report.gdi_advance_pixels > 0);
        assert!(report.owned_advance_milli_pixels > 0);
        assert!(
            fixed_advance_drift_milli_pixels(&report) <= MAX_FIXED_ADVANCE_DRIFT_MILLI_PIXELS,
            "fixed owned advance drifted by {} milli-pixels",
            fixed_advance_drift_milli_pixels(&report)
        );
        assert_eq!(report.retained_mask_count, report.glyph_count);
        assert!(report.retained_pixel_count > 0);
    }

    #[test]
    fn report_schema_is_one_fixed_json_record() {
        let record = format_report(&OwnedTextReport {
            source_bytes: 1,
            glyph_count: 2,
            gdi_advance_pixels: 3,
            owned_advance_milli_pixels: 4,
            retained_mask_count: 5,
            retained_pixel_count: 6,
            first_row_microseconds: 7,
            reused_row_microseconds: 8,
        });
        assert_eq!(
            record,
            concat!(
                "{\"benchmark\":\"anodrel.host.owned-text.v1\",",
                "\"sourceBytes\":1,\"glyphCount\":2,",
                "\"gdiAdvancePixels\":3,\"ownedAdvanceMilliPixels\":4,",
                "\"retainedMaskCount\":5,\"retainedPixelCount\":6,",
                "\"firstRowMicroseconds\":7,\"reusedRowMicroseconds\":8,",
                "\"scope\":\"fixed selected Windows face; local diagnostic only; no visible painter, quality, or release-performance claim\"}"
            )
        );
    }
}
