//! Fixed local evidence comparing current and owned Windows text coverage.
//!
//! This module intentionally compares only four compiled host labels. It is a
//! diagnostic for a later visible-painter decision, not the visible painter.

use std::io;

use anodrel_canvas::{Canvas, Color, Paint, point};
use anodrel_font::FontFace;
use anodrel_glyph::GlyphMaskCache;
use anodrel_text::TextRun;

use super::text::{Align, TextSpec};
use super::{owned_text, selected_font, text};

const CANVAS_WIDTH: u32 = 512;
const CANVAS_HEIGHT: u32 = 96;
const INSET: f32 = 16.0;
const CASES: [ComparisonCase; 4] = [
    ComparisonCase::new("Native Application Platform", 15, 400),
    ComparisonCase::new("Startup Lab", 15, 500),
    ComparisonCase::new("Validated Application", 17, 400),
    ComparisonCase::new("Anodrel", 24, 500),
];

/// One fixed static host label selected for local comparison.
#[derive(Clone, Copy)]
struct ComparisonCase {
    text: &'static str,
    em_pixels: i32,
    weight: i32,
}

impl ComparisonCase {
    const fn new(text: &'static str, em_pixels: i32, weight: i32) -> Self {
        Self {
            text,
            em_pixels,
            weight,
        }
    }
}

/// Closed failures from one full fixed comparison pass.
#[derive(Debug)]
enum ComparisonError {
    Source,
    Face,
    Run,
    Scale,
    Glyph,
    GdiMetric,
    Advance,
    Coverage,
}

/// Closed aggregate facts from the fixed local comparison.
struct ComparisonReport {
    case_count: usize,
    max_advance_drift_milli_pixels: u64,
    min_coverage_overlap_milli: u64,
    gdi_ink_pixels: usize,
    owned_ink_pixels: usize,
    retained_mask_count: usize,
    retained_pixel_count: usize,
}

/// Private coverage facts for one fixed GDI/owned label pair.
struct CaseFacts {
    gdi_advance_milli_pixels: i64,
    owned_advance_milli_pixels: i64,
    coverage_overlap_milli: u64,
    gdi_ink_pixels: usize,
    owned_ink_pixels: usize,
    retained_mask_count: usize,
    retained_pixel_count: usize,
}

/// Prints the fixed comparison report without opening a window or accepting input.
pub(crate) fn run_report() -> io::Result<()> {
    let report =
        inspect().map_err(|_| io::Error::other("owned-text comparison did not complete"))?;
    println!("{}", format_report(&report));
    Ok(())
}

/// Compares every fixed label and retains only the aggregate diagnostic facts.
fn inspect() -> Result<ComparisonReport, ComparisonError> {
    let mut max_advance_drift_milli_pixels = 0_u64;
    let mut min_coverage_overlap_milli = u64::MAX;
    let mut gdi_ink_pixels = 0_usize;
    let mut owned_ink_pixels = 0_usize;
    let mut retained_mask_count = 0_usize;
    let mut retained_pixel_count = 0_usize;

    for case in CASES {
        let facts = compare_case(case)?;
        max_advance_drift_milli_pixels = max_advance_drift_milli_pixels.max(
            u64::try_from(
                i128::from(facts.gdi_advance_milli_pixels)
                    .saturating_sub(i128::from(facts.owned_advance_milli_pixels))
                    .abs(),
            )
            .unwrap_or(u64::MAX),
        );
        min_coverage_overlap_milli = min_coverage_overlap_milli.min(facts.coverage_overlap_milli);
        gdi_ink_pixels = gdi_ink_pixels.saturating_add(facts.gdi_ink_pixels);
        owned_ink_pixels = owned_ink_pixels.saturating_add(facts.owned_ink_pixels);
        retained_mask_count = retained_mask_count.saturating_add(facts.retained_mask_count);
        retained_pixel_count = retained_pixel_count.saturating_add(facts.retained_pixel_count);
    }

    if gdi_ink_pixels == 0 || owned_ink_pixels == 0 || retained_mask_count == 0 {
        return Err(ComparisonError::Coverage);
    }
    Ok(ComparisonReport {
        case_count: CASES.len(),
        max_advance_drift_milli_pixels,
        min_coverage_overlap_milli,
        gdi_ink_pixels,
        owned_ink_pixels,
        retained_mask_count,
        retained_pixel_count,
    })
}

/// Composes one exact current-GDI and owned-selected-face label pair.
fn compare_case(case: ComparisonCase) -> Result<CaseFacts, ComparisonError> {
    let spec = TextSpec::new(case.text, case.em_pixels, case.weight);
    let gdi_advance_milli_pixels =
        milli_pixels(text::width(&spec)).ok_or(ComparisonError::GdiMetric)?;
    let mut gdi = blank_canvas();
    text::draw(
        &mut gdi,
        &spec,
        point(INSET, INSET),
        Align::Left,
        &Paint::solid(Color::WHITE),
    );

    let source = selected_font::selected_face_data(case.em_pixels, case.weight)
        .ok_or(ComparisonError::Source)?;
    let face = FontFace::parse(&source).map_err(|_| ComparisonError::Face)?;
    let run = TextRun::build(&face, case.text).map_err(|_| ComparisonError::Run)?;
    let pixels_per_design_unit = owned_text::pixels_per_design_unit(&run, case.em_pixels as f32)
        .ok_or(ComparisonError::Scale)?;
    let owned_advance_milli_pixels = owned_text::advance_milli_pixels(&run, pixels_per_design_unit)
        .ok_or(ComparisonError::Advance)?;
    let baseline = point(
        INSET,
        INSET + f32::from(run.metrics().ascender()) * pixels_per_design_unit,
    );
    let mut owned = blank_canvas();
    let mut cache = GlyphMaskCache::new(&face);
    owned_text::draw_row(
        &mut owned,
        &mut cache,
        &run,
        pixels_per_design_unit,
        baseline,
        &Paint::solid(Color::WHITE),
    )
    .map_err(|_| ComparisonError::Glyph)?;

    let gdi_bounds = ink_bounds(&gdi).ok_or(ComparisonError::Coverage)?;
    let owned_bounds = ink_bounds(&owned).ok_or(ComparisonError::Coverage)?;
    Ok(CaseFacts {
        gdi_advance_milli_pixels,
        owned_advance_milli_pixels,
        coverage_overlap_milli: aligned_coverage_overlap_milli(
            &gdi,
            gdi_bounds,
            &owned,
            owned_bounds,
        ),
        gdi_ink_pixels: ink_pixels(&gdi),
        owned_ink_pixels: ink_pixels(&owned),
        retained_mask_count: cache.retained_mask_count(),
        retained_pixel_count: cache.retained_pixel_count(),
    })
}

/// Builds a fixed opaque target where white coverage is easy to inspect.
fn blank_canvas() -> Canvas {
    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);
    canvas.clear(Color::BLACK);
    canvas
}

/// Converts a positive finite GDI advance to thousandths of a physical pixel.
fn milli_pixels(value: f32) -> Option<i64> {
    let milli_pixels = f64::from(value) * 1_000.0;
    (milli_pixels.is_finite() && (0.0..=i64::MAX as f64).contains(&milli_pixels))
        .then(|| milli_pixels.round() as i64)
}

/// One tight non-background rectangle in canvas coordinates.
#[derive(Clone, Copy)]
struct InkBounds {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl InkBounds {
    const fn width(self) -> u32 {
        self.right - self.left
    }

    const fn height(self) -> u32 {
        self.bottom - self.top
    }
}

/// Finds the tight rectangle around non-background text coverage.
fn ink_bounds(canvas: &Canvas) -> Option<InkBounds> {
    let mut left = canvas.width();
    let mut top = canvas.height();
    let mut right = 0;
    let mut bottom = 0;
    for y in 0..canvas.height() {
        for x in 0..canvas.width() {
            if coverage(canvas, x, y) == 0 {
                continue;
            }
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    (left < right && top < bottom).then_some(InkBounds {
        left,
        top,
        right,
        bottom,
    })
}

/// Counts coverage pixels, preserving the distinction from total alpha weight.
fn ink_pixels(canvas: &Canvas) -> usize {
    canvas
        .pixels()
        .iter()
        .filter(|pixel| ((**pixel >> 16) & 0xFF) != 0)
        .count()
}

/// Reads white-on-black text coverage without exposing canvas storage.
fn coverage(canvas: &Canvas, x: u32, y: u32) -> u8 {
    let x = i32::try_from(x).expect("fixed canvas width fits i32");
    let y = i32::try_from(y).expect("fixed canvas height fits i32");
    canvas.pixel(x, y).red
}

/// Measures alpha overlap after independently aligning both tight ink bounds.
fn aligned_coverage_overlap_milli(
    first: &Canvas,
    first_bounds: InkBounds,
    second: &Canvas,
    second_bounds: InkBounds,
) -> u64 {
    let width = first_bounds.width().max(second_bounds.width());
    let height = first_bounds.height().max(second_bounds.height());
    let mut shared = 0_u64;
    let mut union = 0_u64;
    for y in 0..height {
        for x in 0..width {
            let first_coverage = coverage_within(first, first_bounds, x, y);
            let second_coverage = coverage_within(second, second_bounds, x, y);
            shared += u64::from(first_coverage.min(second_coverage));
            union += u64::from(first_coverage.max(second_coverage));
        }
    }
    shared.saturating_mul(1_000) / union.max(1)
}

/// Reads one aligned coverage value, treating space outside its ink bounds as empty.
fn coverage_within(canvas: &Canvas, bounds: InkBounds, x: u32, y: u32) -> u8 {
    if x < bounds.width() && y < bounds.height() {
        coverage(canvas, bounds.left + x, bounds.top + y)
    } else {
        0
    }
}

/// Encodes the closed report vocabulary without accepting diagnostic input.
fn format_report(report: &ComparisonReport) -> String {
    format!(
        concat!(
            "{{\"benchmark\":\"anodrel.host.owned-text-comparison.v1\",",
            "\"caseCount\":{},\"maxAdvanceDriftMilliPixels\":{},",
            "\"minCoverageOverlapMilli\":{},\"gdiInkPixels\":{},",
            "\"ownedInkPixels\":{},\"retainedMaskCount\":{},",
            "\"retainedPixelCount\":{},",
            "\"scope\":\"fixed selected Windows labels; local diagnostic only; no visible painter, quality, or release-performance claim\"}}"
        ),
        report.case_count,
        report.max_advance_drift_milli_pixels,
        report.min_coverage_overlap_milli,
        report.gdi_ink_pixels,
        report.owned_ink_pixels,
        report.retained_mask_count,
        report.retained_pixel_count,
    )
}

#[cfg(test)]
mod tests {
    use super::{CASES, ComparisonReport, format_report, inspect};

    #[test]
    fn fixed_comparison_completes_with_ink_and_retained_owned_masks() {
        let report = inspect().expect("fixed owned-text comparison completes");
        assert_eq!(report.case_count, CASES.len());
        assert!(report.gdi_ink_pixels > 0);
        assert!(report.owned_ink_pixels > 0);
        assert!(report.retained_mask_count > 0);
        assert!(report.retained_pixel_count > 0);
        assert!(report.min_coverage_overlap_milli > 0);
    }

    #[test]
    fn comparison_report_schema_is_one_fixed_json_record() {
        let record = format_report(&ComparisonReport {
            case_count: 1,
            max_advance_drift_milli_pixels: 2,
            min_coverage_overlap_milli: 3,
            gdi_ink_pixels: 4,
            owned_ink_pixels: 5,
            retained_mask_count: 6,
            retained_pixel_count: 7,
        });
        assert_eq!(
            record,
            concat!(
                "{\"benchmark\":\"anodrel.host.owned-text-comparison.v1\",",
                "\"caseCount\":1,\"maxAdvanceDriftMilliPixels\":2,",
                "\"minCoverageOverlapMilli\":3,\"gdiInkPixels\":4,",
                "\"ownedInkPixels\":5,\"retainedMaskCount\":6,",
                "\"retainedPixelCount\":7,",
                "\"scope\":\"fixed selected Windows labels; local diagnostic only; no visible painter, quality, or release-performance claim\"}"
            )
        );
    }
}
