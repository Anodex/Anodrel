//! Direct bounded client-size handling for one known session window.
//!
//! The caller resolves the session and decides whether reversible fullscreen is
//! active. This module accepts only an already-validated logical client size;
//! it neither parses a protocol message nor returns a native rectangle.

use super::{Bool, Dword, Hwnd, Rect, Uint};
use anodrel_window::WindowSize;

const GWL_STYLE: i32 = -16;
const GWL_EXSTYLE: i32 = -20;
const LOGICAL_DPI: u32 = 96;
const SWP_NOMOVE: Uint = 0x0002;
const SWP_NOZORDER: Uint = 0x0004;
const SWP_NOACTIVATE: Uint = 0x0010;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> Dword;
    fn SetLastError(error: Dword);
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetWindowLongPtrW(window: Hwnd, index: i32) -> isize;
    fn GetDpiForWindow(window: Hwnd) -> Uint;
    fn GetMenu(window: Hwnd) -> isize;
    fn AdjustWindowRectExForDpi(
        rectangle: *mut Rect,
        style: Dword,
        has_menu: Bool,
        extended_style: Dword,
        dpi: Uint,
    ) -> Bool;
    fn SetWindowPos(
        window: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: Uint,
    ) -> Bool;
}

/// Applies one bounded logical client size without moving or activating `window`.
///
/// It derives the host-selected outer frame at the known window's current DPI.
/// A false result contains no native error, frame, DPI, monitor, or geometry
/// detail for callers to expose.
pub(super) fn apply(window: Hwnd, size: WindowSize) -> bool {
    let Some(style) = window_long(window, GWL_STYLE) else {
        return false;
    };
    let Some(extended_style) = window_long(window, GWL_EXSTYLE) else {
        return false;
    };
    // SAFETY: `window` is host-created on its owning UI thread. The DPI remains
    // private input to the native conversion; zero is the documented invalid
    // result and is rejected below.
    let dpi = unsafe { GetDpiForWindow(window) };
    if dpi == 0 {
        return false;
    }
    let Some(client_width) = logical_to_physical(size.width(), dpi) else {
        return false;
    };
    let Some(client_height) = logical_to_physical(size.height(), dpi) else {
        return false;
    };
    let mut rectangle = Rect::new(0, 0, client_width, client_height);
    // SAFETY: `rectangle` is writable stack storage containing the requested
    // bounded client rectangle. Styles, menu presence, and DPI all come from
    // this same host-created window; no application-supplied native value is
    // used.
    let adjusted = unsafe {
        AdjustWindowRectExForDpi(
            &mut rectangle,
            style as Dword,
            if has_menu(window) { 1 } else { 0 },
            extended_style as Dword,
            dpi,
        )
    } != 0;
    if !adjusted {
        return false;
    }
    let Some((width, height)) = rectangle_size(rectangle) else {
        return false;
    };
    // SAFETY: this runs on the thread that created `window`. The dimensions
    // came only from the bounded client request plus User32's frame adjustment;
    // flags preserve the existing position, z-order, and activation.
    (unsafe {
        SetWindowPos(
            window,
            0,
            0,
            0,
            width,
            height,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    }) != 0
}

fn window_long(window: Hwnd, index: i32) -> Option<isize> {
    // A zero style is valid, so distinguish it from failure with the documented
    // thread error pattern before the immediate User32 getter.
    unsafe {
        SetLastError(0);
    }
    // SAFETY: `window` belongs to this host and `index` reads one documented
    // style word; callers never provide a native handle or index.
    let value = unsafe { GetWindowLongPtrW(window, index) };
    // SAFETY: this reads the thread error set by the immediately preceding
    // getter on the same UI thread.
    (value != 0 || unsafe { GetLastError() } == 0).then_some(value)
}

fn has_menu(window: Hwnd) -> bool {
    // SAFETY: this asks User32 only whether the known host window currently has
    // a menu so frame adjustment includes it. The result is not exposed.
    unsafe { GetMenu(window) != 0 }
}

fn logical_to_physical(logical: u32, dpi: u32) -> Option<i32> {
    let physical = u64::from(logical)
        .checked_mul(u64::from(dpi))?
        .checked_add(u64::from(LOGICAL_DPI / 2))?
        / u64::from(LOGICAL_DPI);
    i32::try_from(physical).ok().filter(|value| *value > 0)
}

fn rectangle_size(rectangle: Rect) -> Option<(i32, i32)> {
    let width = rectangle.right.checked_sub(rectangle.left)?;
    let height = rectangle.bottom.checked_sub(rectangle.top)?;
    (width > 0 && height > 0).then_some((width, height))
}

#[cfg(test)]
mod tests {
    use super::{logical_to_physical, rectangle_size};
    use crate::win32::Rect;

    #[test]
    fn logical_client_dimensions_scale_at_the_known_window_dpi() {
        assert_eq!(logical_to_physical(800, 96), Some(800));
        assert_eq!(logical_to_physical(800, 120), Some(1_000));
        assert_eq!(logical_to_physical(800, 144), Some(1_200));
        assert_eq!(logical_to_physical(333, 120), Some(416));
    }

    #[test]
    fn frame_adjustment_result_requires_positive_outer_dimensions() {
        assert_eq!(
            rectangle_size(Rect::new(-8, -31, 808, 608)),
            Some((816, 639))
        );
        assert_eq!(rectangle_size(Rect::new(10, 10, 10, 20)), None);
        assert_eq!(rectangle_size(Rect::new(10, 20, 30, 20)), None);
    }
}
