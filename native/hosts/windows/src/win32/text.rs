//! Text as canvas coverage.
//!
//! Windows owns font files, shaping, and hinting, so the host asks GDI to draw
//! glyphs into a private monochrome bitmap and lifts the result out as a
//! coverage mask. Everything after that is canvas compositing.
//!
//! Routing text through the canvas instead of drawing it onto the window device
//! context buys three things: text can be filled with a gradient, it can carry
//! real opacity during a reveal, and the entire frame — graphics and type — is
//! still one bitmap reaching the screen in one blit.

use std::{cell::RefCell, collections::HashMap, ptr, rc::Rc};

use anodrel_canvas::{Canvas, Mask, Paint, Point};

use super::present::BitmapInfo;
use super::{Bool, Dword, Hdc, Uint};

type Hgdiobj = isize;
type Hbitmap = isize;

const TRANSPARENT: i32 = 1;
const DEFAULT_CHARSET: Dword = 1;
const OUT_TT_PRECIS: Dword = 4;
const CLIP_DEFAULT_PRECIS: Dword = 0;
const ANTIALIASED_QUALITY: Dword = 4;
const DEFAULT_PITCH: Dword = 0;
const DIB_RGB_COLORS: Uint = 0;

/// Padding around a rendered run so antialiased edges are never clipped.
const GLYPH_PADDING: i32 = 3;

/// The typeface every first-party surface uses.
const FACE_NAME: &str = "Segoe UI";

#[repr(C)]
#[derive(Default)]
struct Size {
    cx: i32,
    cy: i32,
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(device_context: Hdc) -> Hdc;
    fn DeleteDC(device_context: Hdc) -> Bool;
    fn CreateDIBSection(
        device_context: Hdc,
        bitmap_info: *const BitmapInfo,
        usage: Uint,
        bits: *mut *mut core::ffi::c_void,
        section: isize,
        offset: Dword,
    ) -> Hbitmap;
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: Dword,
        underline: Dword,
        strike_out: Dword,
        char_set: Dword,
        output_precision: Dword,
        clip_precision: Dword,
        quality: Dword,
        pitch_and_family: Dword,
        face_name: *const u16,
    ) -> Hgdiobj;
    fn SelectObject(device_context: Hdc, object: Hgdiobj) -> Hgdiobj;
    fn DeleteObject(object: Hgdiobj) -> Bool;
    fn SetTextColor(device_context: Hdc, color: Dword) -> Dword;
    fn SetBkMode(device_context: Hdc, mode: i32) -> i32;
    fn SetTextCharacterExtra(device_context: Hdc, extra: i32) -> i32;
    fn GetTextExtentPoint32W(
        device_context: Hdc,
        text: *const u16,
        count: i32,
        size: *mut Size,
    ) -> Bool;
    fn TextOutW(device_context: Hdc, x: i32, y: i32, text: *const u16, count: i32) -> Bool;
    fn GdiFlush() -> Bool;
}

/// A run of text and the typographic settings it is rendered with.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) struct TextSpec {
    text: String,
    size: i32,
    weight: i32,
    tracking: i32,
}

impl TextSpec {
    /// Builds a run. `size` is the em height in pixels.
    pub(super) fn new(text: impl Into<String>, size: i32, weight: i32) -> Self {
        Self {
            text: text.into(),
            size: size.max(1),
            weight,
            tracking: 0,
        }
    }

    /// Returns a copy with extra spacing between characters, in pixels.
    pub(super) fn tracked(self, tracking: i32) -> Self {
        Self { tracking, ..self }
    }
}

/// Horizontal placement relative to an anchor point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Align {
    /// The anchor is the run's left edge.
    Left,
    /// The anchor is the run's horizontal centre.
    Center,
    /// The anchor is the run's right edge.
    Right,
}

/// Glyph coverage lifted out of GDI, in `0.0..=1.0`.
pub(super) struct GlyphMask {
    /// Origin-zero reusable coverage from GDI's one rasterization pass.
    mask: Mask,
    /// Distance from the mask's left edge to the run's first pixel.
    inset_x: i32,
    /// Distance from the mask's top edge to the run's first pixel.
    inset_y: i32,
    advance: i32,
}

/// The advance and line height of a run, without its pixels.
#[derive(Clone, Copy)]
struct Metrics {
    advance: i32,
    line_height: i32,
}

/// Cache ceiling, past which a cache is emptied rather than grown.
///
/// One layout uses a few dozen runs. Resizing a window changes every font size,
/// so entries accumulate across a drag; clearing wholesale is the simplest
/// bound that cannot leak, and the next frame repopulates what it needs.
const MAX_CACHED_RUNS: usize = 512;

thread_local! {
    /// Rendered runs, keyed by their spec.
    ///
    /// A reveal repaints the same strings many times per second while only
    /// their colour and position change, so rasterizing each run once is the
    /// difference between a smooth reveal and a stuttering one.
    static GLYPHS: RefCell<HashMap<TextSpec, Option<Rc<GlyphMask>>>> =
        RefCell::new(HashMap::new());

    /// Measurements, kept separately from pixels.
    ///
    /// Word wrapping probes a candidate line per word. Measuring through the
    /// glyph cache would rasterize — and then retain — every prefix of every
    /// paragraph, so measurement takes a path that never touches a bitmap.
    static METRICS: RefCell<HashMap<TextSpec, Option<Metrics>>> =
        RefCell::new(HashMap::new());
}

fn cached<K, V>(cache: &RefCell<HashMap<K, V>>, key: &K, produce: impl FnOnce() -> V) -> V
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    if let Some(existing) = cache.borrow().get(key) {
        return existing.clone();
    }
    let produced = produce();
    let mut entries = cache.borrow_mut();
    if entries.len() >= MAX_CACHED_RUNS {
        entries.clear();
    }
    entries.insert(key.clone(), produced.clone());
    produced
}

/// Returns the cached coverage for a run, rendering it on first use.
fn glyph_mask(spec: &TextSpec) -> Option<Rc<GlyphMask>> {
    GLYPHS.with(|cache| cached(cache, spec, || render(spec).map(Rc::new)))
}

/// Returns a run's metrics without rasterizing it.
fn metrics(spec: &TextSpec) -> Option<Metrics> {
    METRICS.with(|cache| cached(cache, spec, || measure(spec)))
}

/// Returns the advance width of a run in pixels.
pub(super) fn width(spec: &TextSpec) -> f32 {
    metrics(spec).map_or(0.0, |measured| measured.advance as f32)
}

/// Returns the line height of a run in pixels.
pub(super) fn line_height(spec: &TextSpec) -> f32 {
    metrics(spec).map_or(spec.size as f32 * 1.3, |measured| {
        measured.line_height as f32
    })
}

/// Composites a run into the canvas.
///
/// `at` anchors the run: `x` per `align`, `y` at the top of the line box.
pub(super) fn draw(canvas: &mut Canvas, spec: &TextSpec, at: Point, align: Align, paint: &Paint) {
    let Some(glyphs) = glyph_mask(spec) else {
        return;
    };
    let left = match align {
        Align::Left => at.x,
        Align::Center => at.x - glyphs.advance as f32 / 2.0,
        Align::Right => at.x - glyphs.advance as f32,
    };
    let origin_x = (left.round() as i32) - glyphs.inset_x;
    let origin_y = (at.y.round() as i32) - glyphs.inset_y;
    canvas.fill_mask_offset(&glyphs.mask, origin_x, origin_y, paint);
}

/// Draws a run and returns the x position just past its last glyph.
pub(super) fn draw_run(canvas: &mut Canvas, spec: &TextSpec, at: Point, paint: &Paint) -> f32 {
    draw(canvas, spec, at, Align::Left, paint);
    at.x + width(spec)
}

/// Runs `work` with a memory device context that has `spec`'s font selected.
///
/// Both handles are released on every path, including when `work` returns
/// `None`.
fn with_font<R>(spec: &TextSpec, work: impl FnOnce(Hdc, &Size) -> Option<R>) -> Option<R> {
    if spec.text.is_empty() {
        return None;
    }
    let text: Vec<u16> = spec.text.encode_utf16().collect();
    let face = wide_null(FACE_NAME);

    // SAFETY: every handle created below is released before returning on all
    // paths. The device context is memory-only and never touches the screen.
    unsafe {
        let device_context = CreateCompatibleDC(0);
        if device_context == 0 {
            return None;
        }
        let font = CreateFontW(
            -spec.size,
            0,
            0,
            0,
            spec.weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_TT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            DEFAULT_PITCH,
            face.as_ptr(),
        );
        if font == 0 {
            DeleteDC(device_context);
            return None;
        }
        let previous_font = SelectObject(device_context, font);
        SetTextCharacterExtra(device_context, spec.tracking);

        let mut extent = Size::default();
        let measured = GetTextExtentPoint32W(
            device_context,
            text.as_ptr(),
            text.len() as i32,
            &mut extent,
        );
        let result = if measured == 0 || extent.cx <= 0 || extent.cy <= 0 {
            None
        } else {
            work(device_context, &extent)
        };

        SelectObject(device_context, previous_font);
        DeleteObject(font);
        DeleteDC(device_context);
        result
    }
}

/// Measures a run without allocating a bitmap or rasterizing a glyph.
fn measure(spec: &TextSpec) -> Option<Metrics> {
    with_font(spec, |_, extent| {
        Some(Metrics {
            advance: extent.cx,
            line_height: extent.cy,
        })
    })
}

/// Rasterizes a run through GDI into a private bitmap and lifts out its coverage.
fn render(spec: &TextSpec) -> Option<GlyphMask> {
    let text: Vec<u16> = spec.text.encode_utf16().collect();
    with_font(spec, |device_context, extent| {
        // SAFETY: `with_font` guarantees a memory device context with this
        // spec's font selected, and `extent` is the measurement GDI returned
        // for `text` under that font.
        unsafe { lift_coverage(device_context, &text, spec, extent) }
    })
}

/// Draws the run into a fresh bitmap and copies its grey levels out as coverage.
///
/// # Safety
///
/// `device_context` must be a memory device context with the caller's font
/// already selected, and `extent` must be the measurement GDI returned for
/// `text` under that font.
unsafe fn lift_coverage(
    device_context: Hdc,
    text: &[u16],
    spec: &TextSpec,
    extent: &Size,
) -> Option<GlyphMask> {
    let width = extent.cx + GLYPH_PADDING * 2;
    let height = extent.cy + GLYPH_PADDING * 2;
    let info = BitmapInfo::top_down(width, height);
    let mut bits: *mut core::ffi::c_void = ptr::null_mut();

    // SAFETY: `info` describes a width * height 32-bit top-down surface and
    // outlives the call; `bits` receives the section's pixel pointer.
    let bitmap =
        unsafe { CreateDIBSection(device_context, &info, DIB_RGB_COLORS, &mut bits, 0, 0) };
    if bitmap == 0 || bits.is_null() {
        return None;
    }

    // SAFETY: the section is owned here and released on every path below.
    let previous_bitmap = unsafe { SelectObject(device_context, bitmap) };
    let pixel_count = (width as usize) * (height as usize);
    // SAFETY: CreateDIBSection guarantees `bits` addresses pixel_count 32-bit
    // pixels for as long as the bitmap is alive, which spans this block.
    let pixels = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u32>(), pixel_count) };
    pixels.fill(0);

    // SAFETY: the selected font and bitmap belong to this device context, and
    // `text` stays live for the duration of the call.
    unsafe {
        SetBkMode(device_context, TRANSPARENT);
        SetTextColor(device_context, 0x00FF_FFFF);
        SetTextCharacterExtra(device_context, spec.tracking);
        TextOutW(
            device_context,
            GLYPH_PADDING,
            GLYPH_PADDING,
            text.as_ptr(),
            text.len() as i32,
        );
        // GDI batches drawing; the batch must be flushed before the bitmap's
        // memory reflects it.
        GdiFlush();
    }

    // Antialiased text is drawn white on black, so any channel carries the
    // coverage. The green channel is read because it is byte-aligned in the
    // packed pixel regardless of endianness handling elsewhere.
    let coverage: Vec<f32> = pixels
        .iter()
        .map(|pixel| f32::from(((pixel >> 8) & 0xFF) as u8) / 255.0)
        .collect();

    // SAFETY: restoring the previous selection before deleting the bitmap is
    // required; a selected object cannot be deleted.
    unsafe {
        SelectObject(device_context, previous_bitmap);
        DeleteObject(bitmap);
    }

    let mask = Mask::from_coverage(0, 0, width as u32, height as u32, coverage)?;
    Some(GlyphMask {
        mask,
        inset_x: GLYPH_PADDING,
        inset_y: GLYPH_PADDING,
        advance: extent.cx,
    })
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests;
