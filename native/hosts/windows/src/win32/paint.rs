//! Direct GDI primitives shared by the host-owned window surfaces.

use std::ptr;

use super::{Hdc, Rect};

type Bool = i32;
type ColorRef = u32;
type Dword = u32;
type Hgdobj = isize;

const DT_LEFT: Dword = 0x0000;
const DT_TOP: Dword = 0x0000;
const DT_WORDBREAK: Dword = 0x0010;
const DT_SINGLELINE: Dword = 0x0020;
const TRANSPARENT: i32 = 1;
const PS_SOLID: i32 = 0;
const HOLLOW_BRUSH: i32 = 5;
const NULL_PEN: i32 = 8;
const FW_NORMAL: i32 = 400;
const DEFAULT_CHARSET: Dword = 1;
const OUT_DEFAULT_PRECIS: Dword = 0;
const CLIP_DEFAULT_PRECIS: Dword = 0;
const CLEARTYPE_QUALITY: Dword = 5;
const DEFAULT_PITCH: Dword = 0;

pub(super) const MIDNIGHT: ColorRef = rgb(9, 14, 30);
pub(super) const HEADER: ColorRef = rgb(16, 24, 46);
pub(super) const PANEL: ColorRef = rgb(23, 33, 61);
pub(super) const PANEL_BORDER: ColorRef = rgb(48, 65, 107);
pub(super) const INK: ColorRef = rgb(232, 241, 255);
pub(super) const MUTED: ColorRef = rgb(137, 157, 194);
pub(super) const CYAN: ColorRef = rgb(126, 226, 255);
pub(super) const VIOLET: ColorRef = rgb(161, 134, 255);
pub(super) const READY: ColorRef = rgb(106, 238, 185);

#[link(name = "user32")]
unsafe extern "system" {
    fn FillRect(device_context: Hdc, rectangle: *const Rect, brush: Hgdobj) -> i32;
    fn DrawTextW(
        device_context: Hdc,
        text: *const u16,
        text_length: i32,
        rectangle: *mut Rect,
        format: Dword,
    ) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateSolidBrush(color: ColorRef) -> Hgdobj;
    fn CreatePen(style: i32, width: i32, color: ColorRef) -> Hgdobj;
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
    ) -> Hgdobj;
    fn SelectObject(device_context: Hdc, object: Hgdobj) -> Hgdobj;
    fn DeleteObject(object: Hgdobj) -> Bool;
    fn GetStockObject(object: i32) -> Hgdobj;
    fn SetTextColor(device_context: Hdc, color: ColorRef) -> ColorRef;
    fn SetBkMode(device_context: Hdc, mode: i32) -> i32;
    fn Ellipse(device_context: Hdc, left: i32, top: i32, right: i32, bottom: i32) -> Bool;
    fn MoveToEx(device_context: Hdc, x: i32, y: i32, previous: *mut core::ffi::c_void) -> Bool;
    fn LineTo(device_context: Hdc, x: i32, y: i32) -> Bool;
}

pub(super) const fn rgb(red: u8, green: u8, blue: u8) -> ColorRef {
    u32::from_le_bytes([red, green, blue, 0])
}

pub(super) fn draw_text_surface(device_context: Hdc, rect: Rect, text: &[u16]) {
    fill(device_context, rect, rgb(248, 250, 255));
    let mut text_rect = Rect::new(28, 28, rect.right - 28, rect.bottom - 28);
    draw_wide_text(
        device_context,
        &mut text_rect,
        text,
        18,
        FW_NORMAL,
        rgb(24, 35, 58),
        DT_LEFT | DT_TOP | DT_WORDBREAK,
    );
}

pub(super) fn fill(device_context: Hdc, rect: Rect, color: ColorRef) {
    // SAFETY: a solid brush is selected only by FillRect for this synchronous
    // call and is deleted immediately afterward.
    unsafe {
        let brush = CreateSolidBrush(color);
        if brush != 0 {
            FillRect(device_context, &rect, brush);
            DeleteObject(brush);
        }
    }
}

pub(super) fn line(
    device_context: Hdc,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    width: i32,
    color: ColorRef,
) {
    // SAFETY: the pen is owned for this draw operation and the device context
    // is active for the current WM_PAINT handler.
    unsafe {
        let pen = CreatePen(PS_SOLID, width, color);
        if pen == 0 {
            return;
        }
        let previous = SelectObject(device_context, pen);
        MoveToEx(device_context, from_x, from_y, ptr::null_mut());
        LineTo(device_context, to_x, to_y);
        SelectObject(device_context, previous);
        DeleteObject(pen);
    }
}

pub(super) fn outline_ellipse(device_context: Hdc, rect: Rect, width: i32, color: ColorRef) {
    // SAFETY: the owned pen and stock hollow brush are selected only while
    // drawing the ellipse, then the previous selections are restored.
    unsafe {
        let pen = CreatePen(PS_SOLID, width, color);
        if pen == 0 {
            return;
        }
        let previous_pen = SelectObject(device_context, pen);
        let previous_brush = SelectObject(device_context, GetStockObject(HOLLOW_BRUSH));
        Ellipse(device_context, rect.left, rect.top, rect.right, rect.bottom);
        SelectObject(device_context, previous_brush);
        SelectObject(device_context, previous_pen);
        DeleteObject(pen);
    }
}

pub(super) fn solid_ellipse(device_context: Hdc, rect: Rect, color: ColorRef) {
    // SAFETY: the owned brush and stock null pen are selected only while
    // drawing the ellipse, then the previous selections are restored.
    unsafe {
        let brush = CreateSolidBrush(color);
        if brush == 0 {
            return;
        }
        let previous_pen = SelectObject(device_context, GetStockObject(NULL_PEN));
        let previous_brush = SelectObject(device_context, brush);
        Ellipse(device_context, rect.left, rect.top, rect.right, rect.bottom);
        SelectObject(device_context, previous_brush);
        SelectObject(device_context, previous_pen);
        DeleteObject(brush);
    }
}

pub(super) fn label(
    device_context: Hdc,
    rect: Rect,
    text: &str,
    height: i32,
    weight: i32,
    color: ColorRef,
    single_line: bool,
) {
    let text = wide_null(text);
    let mut rect = rect;
    let format = if single_line {
        DT_LEFT | DT_TOP | DT_SINGLELINE
    } else {
        DT_LEFT | DT_TOP | DT_WORDBREAK
    };
    draw_wide_text(
        device_context,
        &mut rect,
        &text,
        height,
        weight,
        color,
        format,
    );
}

fn draw_wide_text(
    device_context: Hdc,
    rect: &mut Rect,
    text: &[u16],
    height: i32,
    weight: i32,
    color: ColorRef,
    format: Dword,
) {
    let face_name = wide_null("Segoe UI");
    // SAFETY: the font is local to this synchronous text draw. All pointers
    // remain valid for the duration of the documented GDI calls.
    unsafe {
        let font = CreateFontW(
            -height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH,
            face_name.as_ptr(),
        );
        let previous_font = if font == 0 {
            0
        } else {
            SelectObject(device_context, font)
        };
        let previous_background = SetBkMode(device_context, TRANSPARENT);
        let previous_color = SetTextColor(device_context, color);
        DrawTextW(device_context, text.as_ptr(), -1, rect, format);
        SetTextColor(device_context, previous_color);
        SetBkMode(device_context, previous_background);
        if font != 0 {
            SelectObject(device_context, previous_font);
            DeleteObject(font);
        }
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
