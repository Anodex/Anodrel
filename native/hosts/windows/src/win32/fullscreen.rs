//! Direct reversible borderless-fullscreen handling for one known window.
//!
//! The caller owns session routing and keeps the returned restore facts beside
//! that session's view. This module knows only a host-created window handle;
//! it neither parses a protocol message nor exposes desktop state.

use std::mem;

use super::{Bool, Dword, Hwnd, Point, Rect, Uint, WS_OVERLAPPEDWINDOW};

const GWL_STYLE: i32 = -16;
const MONITOR_DEFAULTTONEAREST: Dword = 2;
const SWP_NOSIZE: Uint = 0x0001;
const SWP_NOMOVE: Uint = 0x0002;
const SWP_NOZORDER: Uint = 0x0004;
const SWP_NOACTIVATE: Uint = 0x0010;
const SWP_FRAMECHANGED: Uint = 0x0020;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WindowPlacement {
    length: Dword,
    flags: Dword,
    show_command: Dword,
    minimum_position: Point,
    maximum_position: Point,
    normal_position: Rect,
}

impl WindowPlacement {
    fn for_query() -> Self {
        Self {
            length: mem::size_of::<Self>() as Dword,
            ..Self::default()
        }
    }
}

#[repr(C)]
#[derive(Default)]
struct MonitorInfo {
    size: Dword,
    monitor: Rect,
    work: Rect,
    flags: Dword,
}

impl MonitorInfo {
    fn for_query() -> Self {
        Self {
            size: mem::size_of::<Self>() as Dword,
            ..Self::default()
        }
    }
}

/// Host-retained native facts used only to restore a framed session window.
///
/// This is deliberately not a window-state value. It has no protocol encoding,
/// no public getter, and no caller-supplied constructor.
#[derive(Clone)]
pub(super) struct FullscreenRestore {
    style: isize,
    placement: WindowPlacement,
}

/// The private result of attempting a fullscreen entry.
///
/// `Windowed` means the transition did not alter the window or was restored
/// successfully. `RestorePending` means the host must retain the captured
/// facts so a later windowed request can repair a partial native transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FullscreenEntry {
    Applied,
    Windowed,
    RestorePending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MonitorBounds {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> Dword;
    fn SetLastError(error: Dword);
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetWindowLongPtrW(window: Hwnd, index: i32) -> isize;
    fn SetWindowLongPtrW(window: Hwnd, index: i32, value: isize) -> isize;
    fn GetWindowPlacement(window: Hwnd, placement: *mut WindowPlacement) -> Bool;
    fn SetWindowPlacement(window: Hwnd, placement: *const WindowPlacement) -> Bool;
    fn MonitorFromWindow(window: Hwnd, flags: Dword) -> isize;
    fn GetMonitorInfoW(monitor: isize, information: *mut MonitorInfo) -> Bool;
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

/// Captures the only native facts needed to restore the known window later.
pub(super) fn capture(window: Hwnd) -> Option<FullscreenRestore> {
    let style = window_style(window)?;
    let mut placement = WindowPlacement::for_query();
    // SAFETY: `placement` is writable stack storage whose `length` is set to
    // the exact Win32 structure size, and `window` belongs to this host.
    if unsafe { GetWindowPlacement(window, &mut placement) } == 0 {
        return None;
    }
    Some(FullscreenRestore { style, placement })
}

/// Applies borderless fullscreen on the monitor Windows associates with `window`.
///
/// Any failure leaves the caller with one of the closed outcomes above. It
/// never returns a native error code or monitor fact.
pub(super) fn enter(window: Hwnd, saved: &FullscreenRestore) -> FullscreenEntry {
    let Some(bounds) = monitor_bounds(window) else {
        return FullscreenEntry::Windowed;
    };
    if !set_window_style(window, borderless_style(saved.style)) {
        return FullscreenEntry::Windowed;
    }
    // SAFETY: this is the host UI thread that created `window`. The rectangle
    // came only from the monitor Windows chose for that same known window, and
    // the flags preserve z-order and activation while refreshing the frame.
    let positioned = unsafe {
        SetWindowPos(
            window,
            0,
            bounds.left,
            bounds.top,
            bounds.width,
            bounds.height,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    } != 0;
    if positioned {
        FullscreenEntry::Applied
    } else if restore(window, saved) {
        FullscreenEntry::Windowed
    } else {
        FullscreenEntry::RestorePending
    }
}

/// Restores a saved framed-window presentation through the matching Win32 API.
///
/// `SetWindowPlacement`, rather than a hand-assembled rectangle, preserves the
/// documented workspace-coordinate semantics of the captured placement.
pub(super) fn restore(window: Hwnd, saved: &FullscreenRestore) -> bool {
    let style_restored = set_window_style(window, saved.style);
    // SAFETY: `saved.placement` came from `GetWindowPlacement` for this host
    // window, retains the required structure length, and outlives the call.
    let placement_restored = unsafe { SetWindowPlacement(window, &saved.placement) } != 0;
    // SAFETY: the window is host-created on this UI thread. This does not move,
    // resize, activate, or reorder it; it asks User32 to recalculate its frame
    // after the saved style and placement were restored.
    let frame_restored = unsafe {
        SetWindowPos(
            window,
            0,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    } != 0;
    style_restored && placement_restored && frame_restored
}

fn window_style(window: Hwnd) -> Option<isize> {
    // A zero style is a valid result, so clear the thread error first and only
    // treat zero as a failure when User32 subsequently set an error.
    unsafe {
        SetLastError(0);
    }
    // SAFETY: `window` belongs to this host and `GWL_STYLE` asks only for its
    // style word; no application-supplied handle is involved.
    let style = unsafe { GetWindowLongPtrW(window, GWL_STYLE) };
    // SAFETY: this reads the thread's error set by the immediately preceding
    // Win32 call on this same UI thread.
    let error = unsafe { GetLastError() };
    (style != 0 || error == 0).then_some(style)
}

fn set_window_style(window: Hwnd, style: isize) -> bool {
    // Like the getter, the previous style may legitimately be zero.
    unsafe {
        SetLastError(0);
    }
    // SAFETY: this host-created window is changed only on its owning UI
    // thread, and `style` comes from a captured host value or a fixed mask.
    let previous = unsafe { SetWindowLongPtrW(window, GWL_STYLE, style) };
    // SAFETY: this reads the thread error corresponding to the write above.
    previous != 0 || unsafe { GetLastError() } == 0
}

fn monitor_bounds(window: Hwnd) -> Option<MonitorBounds> {
    // SAFETY: `window` is the host-owned window whose current monitor Windows
    // is asked to select. The fallback chooses its nearest known monitor.
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    if monitor == 0 {
        return None;
    }
    let mut information = MonitorInfo::for_query();
    // SAFETY: `information` is writable stack storage initialized with the
    // exact required structure size, and `monitor` came from User32 above.
    if unsafe { GetMonitorInfoW(monitor, &mut information) } == 0 {
        return None;
    }
    bounds_from_monitor_rect(information.monitor)
}

fn bounds_from_monitor_rect(rectangle: Rect) -> Option<MonitorBounds> {
    let width = rectangle.width();
    let height = rectangle.height();
    (width > 0 && height > 0).then_some(MonitorBounds {
        left: rectangle.left,
        top: rectangle.top,
        width,
        height,
    })
}

const fn borderless_style(style: isize) -> isize {
    style & !(WS_OVERLAPPEDWINDOW as isize)
}

#[cfg(test)]
mod tests {
    use super::{MonitorBounds, WS_OVERLAPPEDWINDOW, borderless_style, bounds_from_monitor_rect};
    use crate::win32::Rect;

    #[test]
    fn borderless_style_removes_only_the_standard_framed_window_bits() {
        const WS_VISIBLE: isize = 0x1000_0000;
        const WS_CLIPCHILDREN: isize = 0x0200_0000;
        let style = WS_OVERLAPPEDWINDOW as isize | WS_VISIBLE | WS_CLIPCHILDREN;
        assert_eq!(borderless_style(style), WS_VISIBLE | WS_CLIPCHILDREN);
    }

    #[test]
    fn monitor_bounds_preserve_a_non_primary_monitor_origin() {
        assert_eq!(
            bounds_from_monitor_rect(Rect::new(-1_920, 0, 0, 1_080)),
            Some(MonitorBounds {
                left: -1_920,
                top: 0,
                width: 1_920,
                height: 1_080,
            })
        );
    }

    #[test]
    fn monitor_bounds_reject_a_degenerate_native_rectangle() {
        assert_eq!(bounds_from_monitor_rect(Rect::new(10, 10, 10, 20)), None);
    }
}
