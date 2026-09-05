//! Raw Win32 declarations and shared message-routing values.

#![allow(non_snake_case)]

pub(super) type Atom = u16;
pub(super) type Bool = i32;
pub(super) type Dword = u32;
pub(super) type Hbrush = isize;
pub(super) type Hcursor = isize;
pub(super) type Hdc = isize;
pub(super) type Hinstance = isize;
pub(super) type Hmenu = isize;
pub(super) type Hwnd = isize;
pub(super) type Lparam = isize;
pub(super) type Lresult = isize;
pub(super) type Uint = u32;
pub(super) type Wparam = usize;

pub(super) const CS_HREDRAW: Uint = 0x0002;
pub(super) const CS_VREDRAW: Uint = 0x0001;
pub(super) const WS_OVERLAPPEDWINDOW: Dword = 0x00CF_0000;
pub(super) const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
pub(super) const SW_MAXIMIZE: i32 = 3;
pub(super) const SW_SHOW: i32 = 5;
pub(super) const SW_MINIMIZE: i32 = 6;
pub(super) const SW_RESTORE: i32 = 9;
pub(super) const WM_DESTROY: Uint = 0x0002;
pub(super) const WM_NULL: Uint = 0x0000;
pub(super) const WM_CLOSE: Uint = 0x0010;
pub(super) const WM_PAINT: Uint = 0x000F;
pub(super) const WM_ERASEBKGND: Uint = 0x0014;
pub(super) const WM_SETTINGCHANGE: Uint = 0x001A;
pub(super) const WM_GETMINMAXINFO: Uint = 0x0024;
pub(super) const WM_CONTEXTMENU: Uint = 0x007B;
pub(super) const WM_SETICON: Uint = 0x0080;
pub(super) const WM_COMMAND: Uint = 0x0111;
pub(super) const WM_SYSCOMMAND: Uint = 0x0112;
pub(super) const WM_SETCURSOR: Uint = 0x0020;
pub(super) const WM_GETOBJECT: Uint = 0x003D;
pub(super) const WM_KEYDOWN: Uint = 0x0100;
pub(super) const WM_MOUSEWHEEL: Uint = 0x020A;
pub(super) const WM_MOUSEMOVE: Uint = 0x0200;
pub(super) const WM_LBUTTONDOWN: Uint = 0x0201;
pub(super) const WM_LBUTTONUP: Uint = 0x0202;
pub(super) const WM_RBUTTONUP: Uint = 0x0205;
pub(super) const WM_CAPTURECHANGED: Uint = 0x0215;
pub(super) const WM_MOUSELEAVE: Uint = 0x02A3;
pub(super) const WM_TIMER: Uint = 0x0113;
pub(super) const WM_DPICHANGED: Uint = 0x02E0;
pub(super) const WM_ACTIVATE: Uint = 0x0006;
pub(super) const WM_SIZE: Uint = 0x0005;
pub(super) const WA_INACTIVE: Wparam = 0;
pub(super) const SIZE_MINIMIZED: Wparam = 1;
pub(super) const IDC_ARROW: usize = 32_512;
pub(super) const IDC_HAND: usize = 32_649;
pub(super) const ICON_SMALL: Wparam = 0;
pub(super) const ICON_BIG: Wparam = 1;
pub(super) const TME_LEAVE: Dword = 0x0000_0002;
pub(super) const SWP_NOZORDER: Uint = 0x0004;
pub(super) const SWP_NOSIZE: Uint = 0x0001;
pub(super) const SWP_NOMOVE: Uint = 0x0002;
pub(super) const SWP_NOACTIVATE: Uint = 0x0010;
pub(super) const HWND_TOPMOST: Hwnd = -1;
pub(super) const HTCLIENT: isize = 1;
pub(super) const VK_SHIFT: i32 = 0x10;
pub(super) const VK_CONTROL: i32 = 0x11;
pub(super) const VK_MENU: i32 = 0x12;
pub(super) const VK_TAB: Wparam = 0x09;
pub(super) const VK_RETURN: Wparam = 0x0D;
pub(super) const VK_PRIOR: Wparam = 0x21;
pub(super) const VK_NEXT: Wparam = 0x22;
pub(super) const VK_END: Wparam = 0x23;
pub(super) const VK_HOME: Wparam = 0x24;
pub(super) const VK_LEFT: Wparam = 0x25;
pub(super) const VK_RIGHT: Wparam = 0x27;
pub(super) const VK_DELETE: Wparam = 0x2E;

/// A typed character, already translated from a key by `TranslateMessage`.
///
/// Using `WM_CHAR` rather than decoding `WM_KEYDOWN` is what makes a field work
/// with a keyboard layout the host knows nothing about: Windows has already
/// applied the layout, dead keys, and modifiers by the time this arrives.
pub(super) const WM_CHAR: Uint = 0x0102;

/// Backspace arrives as a control character through `WM_CHAR`, not as an edit
/// key, so it is named here to be recognised and routed as one.
pub(super) const CHAR_BACKSPACE: u32 = 0x08;

/// Bit 30 in a `WM_KEYDOWN` `lParam`: the key was already down before this
/// message, so the message is an auto-repeat rather than a fresh activation.
pub(super) const KEY_WAS_DOWN: Lparam = 1_isize << 30;

/// Private message telling the Startup Lab that a product-session start
/// attempt has finished. It carries no payload: the started session, if any, is
/// collected from the host-owned slot in [`product_tile`].
pub(super) const WM_APP: Uint = 0x8000;
pub(super) const WM_ANODREL_PRODUCT_SESSION: Uint = WM_APP + 1;
/// Private payload-free wakeup for one pending UI Automation focus request.
///
/// A route carries the target in host-owned memory; this message carries no
/// pointer or input data, so an externally posted copy cannot inject focus.
pub(super) const WM_ANODREL_UIA_FOCUS: Uint = WM_APP + 2;
/// Private payload-free wakeup for one pending UI Automation scroll request.
///
/// The selected viewport and closed command stay in host-owned route memory;
/// an externally posted copy cannot select a target or inject scroll data.
pub(super) const WM_ANODREL_UIA_SCROLL: Uint = WM_APP + 3;
/// Private callback for the one host-created notification-area entry.
///
/// Shell32 chooses the local mouse-message payload. The application never
/// supplies this number or any callback value.
pub(super) const WM_ANODREL_NOTIFICATION_AREA: Uint = WM_APP + 4;

/// Timer driving the Startup Lab's reveal, at roughly 60 frames per second.
pub(super) const REVEAL_TIMER: usize = 1;
pub(super) const REVEAL_INTERVAL_MILLIS: Uint = 16;
pub(super) const UI_SESSION_TIMER: usize = 2;
pub(super) const UI_SESSION_POLL_INTERVAL_MILLIS: Uint = 50;

/// Interval the surface settles to once the reveal completes.
///
/// Ambient motion is slow and confined to the mark, so it needs far fewer
/// frames than the reveal. At 30 per second it repaints a region rather than a
/// surface, which is what keeps a living screen from costing a busy one.
pub(super) const AMBIENT_INTERVAL_MILLIS: Uint = 33;

/// Per-monitor DPI awareness, version 2.
pub(super) const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
pub(super) const USER_DEFAULT_SCREEN_DPI: u32 = 96;

/// Smallest client area the layout is designed to remain legible in.
pub(super) const MIN_CLIENT_WIDTH: i32 = 900;
pub(super) const MIN_CLIENT_HEIGHT: i32 = 660;

pub(super) type WndProc = unsafe extern "system" fn(Hwnd, Uint, Wparam, Lparam) -> Lresult;

#[repr(C)]
pub(super) struct WndClassW {
    pub(super) style: Uint,
    pub(super) lpfnWndProc: Option<WndProc>,
    pub(super) cbClsExtra: i32,
    pub(super) cbWndExtra: i32,
    pub(super) hInstance: Hinstance,
    pub(super) hIcon: isize,
    pub(super) hCursor: Hcursor,
    pub(super) hbrBackground: Hbrush,
    pub(super) lpszMenuName: *const u16,
    pub(super) lpszClassName: *const u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Point {
    pub(super) x: i32,
    pub(super) y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Rect {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) right: i32,
    pub(super) bottom: i32,
}

impl Rect {
    pub(super) const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub(super) const fn width(self) -> i32 {
        self.right - self.left
    }

    pub(super) const fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[repr(C)]
#[derive(Default)]
pub(super) struct PaintStruct {
    pub(super) hdc: Hdc,
    pub(super) fErase: Bool,
    pub(super) rcPaint: Rect,
    pub(super) fRestore: Bool,
    pub(super) fIncUpdate: Bool,
    pub(super) rgbReserved: [u8; 32],
}

#[repr(C)]
#[derive(Default)]
pub(super) struct Msg {
    pub(super) hwnd: Hwnd,
    pub(super) message: Uint,
    pub(super) wParam: Wparam,
    pub(super) lParam: Lparam,
    pub(super) time: Dword,
    pub(super) pt: Point,
    pub(super) lPrivate: Dword,
}

#[repr(C)]
#[derive(Default)]
pub(super) struct MinMaxInfo {
    pub(super) reserved: Point,
    pub(super) maxSize: Point,
    pub(super) maxPosition: Point,
    pub(super) minTrackSize: Point,
    pub(super) maxTrackSize: Point,
}

#[repr(C)]
pub(super) struct TrackMouseEventStruct {
    pub(super) cbSize: Dword,
    pub(super) dwFlags: Dword,
    pub(super) hwndTrack: Hwnd,
    pub(super) dwHoverTime: Dword,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub(super) fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
    pub(super) fn GetProcAddress(module: Hinstance, name: *const u8) -> *const core::ffi::c_void;
}

#[link(name = "user32")]
unsafe extern "system" {
    pub(super) fn RegisterClassW(window_class: *const WndClassW) -> Atom;
    pub(super) fn CreateWindowExW(
        extended_style: Dword,
        class_name: *const u16,
        window_name: *const u16,
        style: Dword,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: isize,
        instance: Hinstance,
        parameter: *const core::ffi::c_void,
    ) -> Hwnd;
    pub(super) fn DefWindowProcW(
        window: Hwnd,
        message: Uint,
        wparam: Wparam,
        lparam: Lparam,
    ) -> Lresult;
    pub(super) fn ShowWindow(window: Hwnd, command: i32) -> Bool;
    pub(super) fn IsIconic(window: Hwnd) -> Bool;
    pub(super) fn IsZoomed(window: Hwnd) -> Bool;
    pub(super) fn UpdateWindow(window: Hwnd) -> Bool;
    pub(super) fn SetForegroundWindow(window: Hwnd) -> Bool;
    pub(super) fn DestroyWindow(window: Hwnd) -> Bool;
    pub(super) fn GetSystemMenu(window: Hwnd, revert: Bool) -> Hmenu;
    pub(super) fn CreateMenu() -> Hmenu;
    pub(super) fn CreatePopupMenu() -> Hmenu;
    pub(super) fn AppendMenuW(menu: Hmenu, flags: Uint, new_item: usize, text: *const u16) -> Bool;
    pub(super) fn TrackPopupMenu(
        menu: Hmenu,
        flags: Uint,
        x: i32,
        y: i32,
        reserved: i32,
        window: Hwnd,
        exclusion: *const Rect,
    ) -> Uint;
    pub(super) fn SetMenu(window: Hwnd, menu: Hmenu) -> Bool;
    pub(super) fn DrawMenuBar(window: Hwnd) -> Bool;
    pub(super) fn EnableMenuItem(menu: Hmenu, item: Uint, flags: Uint) -> Uint;
    pub(super) fn DestroyMenu(menu: Hmenu) -> Bool;
    pub(super) fn SetWindowTextW(window: Hwnd, text: *const u16) -> Bool;
    pub(super) fn MessageBoxW(
        owner: Hwnd,
        text: *const u16,
        caption: *const u16,
        style: Uint,
    ) -> i32;
    pub(super) fn GetMessageW(
        message: *mut Msg,
        window: Hwnd,
        minimum: Uint,
        maximum: Uint,
    ) -> Bool;
    pub(super) fn TranslateMessage(message: *const Msg) -> Bool;
    pub(super) fn DispatchMessageW(message: *const Msg) -> Lresult;
    pub(super) fn PostQuitMessage(exit_code: i32);
    pub(super) fn BeginPaint(window: Hwnd, paint: *mut PaintStruct) -> Hdc;
    pub(super) fn EndPaint(window: Hwnd, paint: *const PaintStruct) -> Bool;
    pub(super) fn GetClientRect(window: Hwnd, rectangle: *mut Rect) -> Bool;
    pub(super) fn ClientToScreen(window: Hwnd, point: *mut Point) -> Bool;
    pub(super) fn GetCursorPos(point: *mut Point) -> Bool;
    pub(super) fn LoadCursorW(instance: Hinstance, cursor_name: *const u16) -> Hcursor;
    pub(super) fn SetCursor(cursor: Hcursor) -> Hcursor;
    pub(super) fn SetCapture(window: Hwnd) -> Hwnd;
    pub(super) fn ReleaseCapture() -> Bool;
    pub(super) fn GetKeyState(virtual_key: i32) -> i16;
    pub(super) fn InvalidateRect(window: Hwnd, rectangle: *const Rect, erase: Bool) -> Bool;
    pub(super) fn SendMessageW(
        window: Hwnd,
        message: Uint,
        wparam: Wparam,
        lparam: Lparam,
    ) -> Lresult;
    pub(super) fn PostMessageW(window: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam)
    -> Bool;
    pub(super) fn SetTimer(window: Hwnd, id: usize, elapse: Uint, callback: usize) -> usize;
    pub(super) fn KillTimer(window: Hwnd, id: usize) -> Bool;
    pub(super) fn TrackMouseEvent(track: *mut TrackMouseEventStruct) -> Bool;
    pub(super) fn AdjustWindowRect(rectangle: *mut Rect, style: Dword, menu: Bool) -> Bool;
    pub(super) fn SetWindowPos(
        window: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: Uint,
    ) -> Bool;
}
