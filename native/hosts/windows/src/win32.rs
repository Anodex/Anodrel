//! Direct Win32 window lifecycle and routing for Anodrel surfaces.
//!
//! Raw window management stays here. Everything visible is composed by
//! [`anodrel_canvas`] into a single bitmap and presented in one blit, so this
//! module deals with messages and handles rather than with drawing.
//!
//! Submodules split that responsibility: [`present`] moves a canvas to the
//! screen, [`text`] turns GDI glyphs into canvas coverage, [`appicon`] builds
//! the window icon from brand geometry, and [`startup_lab`], [`document`],
//! [`ui_lab`], and [`window_group_lab`] own the host diagnostic surfaces.
//! [`messages`] contains the contained Win32 callback and its routing rules.
//! [`services`] bridges session mailboxes to host-owned Windows operations.
//! [`launch`] owns the fixed public host-launch routes and message-loop setup.
//! [`startup_views`] owns Startup Lab actions, composition, and ambient paint.

#![allow(non_snake_case)]

mod appicon;
mod crash;
mod document;
mod fullscreen;
mod input;
mod launch;
mod menu;
mod messages;
mod present;
mod product_tile;
mod registry;
mod scrollbar;
mod services;
mod session_window_group;
mod size;
mod startup_lab;
mod startup_views;
mod stats;
mod text;
mod ui_lab;
mod ui_session_view;
mod window_group_lab;

use std::{io, mem, ptr, sync::OnceLock, time::Instant};

use anodrel_canvas::{Canvas, Rect as CanvasRect, point};
use anodrel_core::SessionCloseSignal;
use anodrel_diagnostics::{Event, LogBook};
use anodrel_file_dialog::{FileDialogMailbox, FileDialogRequestKind, FileDialogSelection};
use anodrel_menu::MenuMailbox;
use anodrel_notifications::NotificationMailbox;
use anodrel_ui::UiDocument;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox, UiWindowGroup, UiWindowId};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_instance::PrimaryInstance;

use anodrel_crash::{CrashSite, CrashSurface};
use anodrel_ui_session::UiFieldMailbox;
use anodrel_window::{
    WindowFocusMailbox, WindowFullscreenMailbox, WindowFullscreenMode, WindowSizeMailbox,
    WindowState, WindowStateMailbox, WindowTitleMailbox, WindowTitleProposal,
};

use crate::product::PreflightOutcome;
use document::{Body, Document, Section};
#[cfg(debug_assertions)]
use launch::crash_selftest;
use launch::run_windows;
use messages::{contain_panic, window_proc};
use services::*;
use startup_views::*;

#[cfg(debug_assertions)]
pub use launch::run_crash_selftest_panic;
pub use launch::{
    print_startup_report, run, run_application, run_authenticated_ui_session,
    run_crash_report_selftest, run_grouped_ui_session, run_startup_lab, run_ui_lab, run_ui_preview,
    run_ui_session, run_window_group_lab, run_window_lab,
};
pub use product_tile::FIXTURE_APPLICATION_ID;

type Atom = u16;
pub(super) type Bool = i32;
pub(super) type Dword = u32;
type Hbrush = isize;
type Hcursor = isize;
pub(super) type Hdc = isize;
type Hinstance = isize;
type Hmenu = isize;
type Hwnd = isize;
type Lparam = isize;
type Lresult = isize;
pub(super) type Uint = u32;
type Wparam = usize;

const CS_HREDRAW: Uint = 0x0002;
const CS_VREDRAW: Uint = 0x0001;
const WS_OVERLAPPEDWINDOW: Dword = 0x00CF_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
const SW_MAXIMIZE: i32 = 3;
const SW_SHOW: i32 = 5;
const SW_MINIMIZE: i32 = 6;
const SW_RESTORE: i32 = 9;
const WM_DESTROY: Uint = 0x0002;
const WM_PAINT: Uint = 0x000F;
const WM_ERASEBKGND: Uint = 0x0014;
const WM_SETTINGCHANGE: Uint = 0x001A;
const WM_GETMINMAXINFO: Uint = 0x0024;
const WM_SETICON: Uint = 0x0080;
const WM_COMMAND: Uint = 0x0111;
const WM_SETCURSOR: Uint = 0x0020;
const WM_GETOBJECT: Uint = 0x003D;
const WM_KEYDOWN: Uint = 0x0100;
const WM_MOUSEWHEEL: Uint = 0x020A;
const WM_MOUSEMOVE: Uint = 0x0200;
const WM_LBUTTONDOWN: Uint = 0x0201;
const WM_LBUTTONUP: Uint = 0x0202;
const WM_CAPTURECHANGED: Uint = 0x0215;
const WM_MOUSELEAVE: Uint = 0x02A3;
const WM_TIMER: Uint = 0x0113;
const WM_DPICHANGED: Uint = 0x02E0;
const WM_ACTIVATE: Uint = 0x0006;
const WM_SIZE: Uint = 0x0005;
const WA_INACTIVE: Wparam = 0;
const SIZE_MINIMIZED: Wparam = 1;
const IDC_ARROW: usize = 32_512;
const IDC_HAND: usize = 32_649;
const ICON_SMALL: Wparam = 0;
const ICON_BIG: Wparam = 1;
const TME_LEAVE: Dword = 0x0000_0002;
const SWP_NOZORDER: Uint = 0x0004;
const SWP_NOACTIVATE: Uint = 0x0010;
const HTCLIENT: isize = 1;
const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
const VK_TAB: Wparam = 0x09;
const VK_RETURN: Wparam = 0x0D;
const VK_PRIOR: Wparam = 0x21;
const VK_NEXT: Wparam = 0x22;
const VK_END: Wparam = 0x23;
const VK_HOME: Wparam = 0x24;
const VK_LEFT: Wparam = 0x25;
const VK_RIGHT: Wparam = 0x27;
const VK_DELETE: Wparam = 0x2E;

/// A typed character, already translated from a key by `TranslateMessage`.
///
/// Using `WM_CHAR` rather than decoding `WM_KEYDOWN` is what makes a field work
/// with a keyboard layout the host knows nothing about: Windows has already
/// applied the layout, dead keys, and modifiers by the time this arrives.
const WM_CHAR: Uint = 0x0102;

/// Backspace arrives as a control character through `WM_CHAR`, not as an edit
/// key, so it is named here to be recognised and routed as one.
const CHAR_BACKSPACE: u32 = 0x08;

/// Bit 30 in a `WM_KEYDOWN` `lParam`: the key was already down before this
/// message, so the message is an auto-repeat rather than a fresh activation.
const KEY_WAS_DOWN: Lparam = 1_isize << 30;

/// Private message telling the Startup Lab that a product-session start
/// attempt has finished. It carries no payload: the started session, if any, is
/// collected from the host-owned slot in [`product_tile`].
const WM_APP: Uint = 0x8000;
const WM_ANODREL_PRODUCT_SESSION: Uint = WM_APP + 1;
/// Private payload-free wakeup for one pending UI Automation focus request.
///
/// A route carries the target in host-owned memory; this message carries no
/// pointer or input data, so an externally posted copy cannot inject focus.
const WM_ANODREL_UIA_FOCUS: Uint = WM_APP + 2;
/// Private payload-free wakeup for one pending UI Automation scroll request.
///
/// The selected viewport and closed command stay in host-owned route memory;
/// an externally posted copy cannot select a target or inject scroll data.
const WM_ANODREL_UIA_SCROLL: Uint = WM_APP + 3;

/// Timer driving the Startup Lab's reveal, at roughly 60 frames per second.
const REVEAL_TIMER: usize = 1;
const REVEAL_INTERVAL_MILLIS: Uint = 16;
const UI_SESSION_TIMER: usize = 2;
const UI_SESSION_POLL_INTERVAL_MILLIS: Uint = 50;

/// Interval the surface settles to once the reveal completes.
///
/// Ambient motion is slow and confined to the mark, so it needs far fewer
/// frames than the reveal. At 30 per second it repaints a region rather than a
/// surface, which is what keeps a living screen from costing a busy one.
const AMBIENT_INTERVAL_MILLIS: Uint = 33;

/// Per-monitor DPI awareness, version 2.
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
const USER_DEFAULT_SCREEN_DPI: u32 = 96;

/// Smallest client area the layout is designed to remain legible in.
const MIN_CLIENT_WIDTH: i32 = 900;
const MIN_CLIENT_HEIGHT: i32 = 660;

type WndProc = unsafe extern "system" fn(Hwnd, Uint, Wparam, Lparam) -> Lresult;

#[repr(C)]
struct WndClassW {
    style: Uint,
    lpfnWndProc: Option<WndProc>,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: Hinstance,
    hIcon: isize,
    hCursor: Hcursor,
    hbrBackground: Hbrush,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
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
struct PaintStruct {
    hdc: Hdc,
    fErase: Bool,
    rcPaint: Rect,
    fRestore: Bool,
    fIncUpdate: Bool,
    rgbReserved: [u8; 32],
}

#[repr(C)]
#[derive(Default)]
struct Msg {
    hwnd: Hwnd,
    message: Uint,
    wParam: Wparam,
    lParam: Lparam,
    time: Dword,
    pt: Point,
    lPrivate: Dword,
}

#[repr(C)]
#[derive(Default)]
struct MinMaxInfo {
    reserved: Point,
    maxSize: Point,
    maxPosition: Point,
    minTrackSize: Point,
    maxTrackSize: Point,
}

#[repr(C)]
struct TrackMouseEventStruct {
    cbSize: Dword,
    dwFlags: Dword,
    hwndTrack: Hwnd,
    dwHoverTime: Dword,
}

/// Facts about a validated package that a host surface may display.
///
/// The host copies these out of the application crate so the window layer never
/// holds a package, a filesystem path, or any unvalidated text.
#[derive(Clone)]
pub struct PackageFacts {
    /// Display name from the validated manifest.
    pub display_name: String,
    /// Application identity from the validated manifest.
    pub application_id: String,
    /// Declared content format.
    pub content_format: String,
    /// Package-relative content path, never the resolved filesystem path.
    pub content_path: String,
    /// Verified content digest, lower-case hexadecimal.
    pub content_digest: String,
    /// Number of content bytes that were hashed.
    pub content_bytes: usize,
}

/// Live state behind the Startup Lab surface.
#[derive(Clone)]
pub(super) struct StartupLab {
    pub(super) package: PackageFacts,
    /// Fixed host events recorded after the Startup Lab preflight succeeded.
    log: LogBook,
    /// Time from process start to the surface being ready.
    pub(super) startup_millis: u64,
    pub(super) working_set_bytes: u64,
    /// Cost of the previous frame, reported on the current one.
    pub(super) last_frame_micros: u64,
    revealed_at: Instant,
    /// Index of the action tile under the pointer, if any.
    pub(super) hovered: Option<usize>,
    /// `true` once the reveal has finished and the surface is breathing.
    ambient: bool,
    /// `true` only when a verification-only preflight confirmed, before this
    /// surface opened, that the registered product fixture currently launches.
    ///
    /// This is the single value behind the launch tile's appearance and its
    /// hit-testing, so the surface cannot offer a launch on a machine where the
    /// record or signature does not validate. See `docs/PRODUCT_FIXTURE.md`.
    pub(super) launch_available: bool,
}

/// Builds the diagnostic history displayed by the Startup Lab.
///
/// Its only input is one member of the closed event catalogue, chosen by the
/// caller's preflight. The displayed history therefore still reflects fixed host
/// milestones rather than application text, operating-system errors, paths, or
/// arbitrary caller data.
fn startup_log_book(launch_event: Event) -> LogBook {
    let mut log = LogBook::new();
    // Chronological: the preflight runs alongside the two checks above it and is
    // settled before this surface is authorized to open.
    for event in [
        Event::PackageVerified,
        Event::CoreHealthChecked,
        Event::PipeLoopbackChecked,
        launch_event,
        Event::StartupLabAuthorized,
    ] {
        log.record(event)
            .expect("five fixed startup events fit in the diagnostic log");
    }
    log
}

#[derive(Clone)]
enum View {
    Document(Document),
    StartupLab(StartupLab),
    UiLab(ui_lab::UiLab),
    // The session carries its intentionally private service bridges. Keeping
    // that larger, per-window state behind one allocation keeps every other
    // window definition and registry entry compact.
    UiSession(Box<ui_session_view::UiSessionView>),
}

impl View {
    /// Whether this view must join a session-owned native group before it can
    /// be shown. A legacy UI-session diagnostic intentionally has no group.
    fn requires_group_registration(&self) -> bool {
        matches!(self, Self::UiSession(session) if session.is_group_member())
    }
}

struct WindowDefinition {
    title: String,
    width: i32,
    height: i32,
    view: View,
}

impl WindowDefinition {
    fn document(title: &str, subtitle: &str, text: &str, width: i32, height: i32) -> Self {
        Self {
            title: title.to_owned(),
            width,
            height,
            view: View::Document(Document::from_text(title, subtitle, text)),
        }
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
    fn GetProcAddress(module: Hinstance, name: *const u8) -> *const core::ffi::c_void;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(window_class: *const WndClassW) -> Atom;
    fn CreateWindowExW(
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
    fn DefWindowProcW(window: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam) -> Lresult;
    fn ShowWindow(window: Hwnd, command: i32) -> Bool;
    fn UpdateWindow(window: Hwnd) -> Bool;
    fn SetForegroundWindow(window: Hwnd) -> Bool;
    fn DestroyWindow(window: Hwnd) -> Bool;
    fn CreateMenu() -> Hmenu;
    fn CreatePopupMenu() -> Hmenu;
    fn AppendMenuW(menu: Hmenu, flags: Uint, new_item: usize, text: *const u16) -> Bool;
    fn SetMenu(window: Hwnd, menu: Hmenu) -> Bool;
    fn DrawMenuBar(window: Hwnd) -> Bool;
    fn DestroyMenu(menu: Hmenu) -> Bool;
    fn SetWindowTextW(window: Hwnd, text: *const u16) -> Bool;
    fn GetMessageW(message: *mut Msg, window: Hwnd, minimum: Uint, maximum: Uint) -> Bool;
    fn TranslateMessage(message: *const Msg) -> Bool;
    fn DispatchMessageW(message: *const Msg) -> Lresult;
    fn PostQuitMessage(exit_code: i32);
    fn BeginPaint(window: Hwnd, paint: *mut PaintStruct) -> Hdc;
    fn EndPaint(window: Hwnd, paint: *const PaintStruct) -> Bool;
    fn GetClientRect(window: Hwnd, rectangle: *mut Rect) -> Bool;
    fn ClientToScreen(window: Hwnd, point: *mut Point) -> Bool;
    fn LoadCursorW(instance: Hinstance, cursor_name: *const u16) -> Hcursor;
    fn SetCursor(cursor: Hcursor) -> Hcursor;
    fn SetCapture(window: Hwnd) -> Hwnd;
    fn ReleaseCapture() -> Bool;
    fn GetKeyState(virtual_key: i32) -> i16;
    fn InvalidateRect(window: Hwnd, rectangle: *const Rect, erase: Bool) -> Bool;
    fn SendMessageW(window: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam) -> Lresult;
    fn PostMessageW(window: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam) -> Bool;
    fn SetTimer(window: Hwnd, id: usize, elapse: Uint, callback: usize) -> usize;
    fn KillTimer(window: Hwnd, id: usize) -> Bool;
    fn TrackMouseEvent(track: *mut TrackMouseEventStruct) -> Bool;
    fn AdjustWindowRect(rectangle: *mut Rect, style: Dword, menu: Bool) -> Bool;
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

static ACTIVATION_MESSAGE: OnceLock<Uint> = OnceLock::new();
static WINDOW_CLASS_REGISTERED: OnceLock<()> = OnceLock::new();
static ICONS: OnceLock<(Option<isize>, Option<isize>)> = OnceLock::new();

fn to_wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn module_handle() -> io::Result<Hinstance> {
    // SAFETY: a null module name requests the current process executable, and
    // the returned handle is used only in this process to register a class.
    let handle = unsafe { GetModuleHandleW(ptr::null()) };
    if handle == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(handle)
    }
}

/// Resolves an optional User32 entry point by name.
///
/// The DPI functions used below arrived in later Windows releases. Binding them
/// dynamically keeps the executable loadable where they are absent, at the cost
/// of one lookup.
fn user32_export(name: &[u8]) -> Option<*const core::ffi::c_void> {
    let module_name = to_wide_null("user32.dll");
    // SAFETY: user32 is already loaded in any process with a window; the name
    // is a null-terminated ASCII literal supplied by this module.
    let module = unsafe { GetModuleHandleW(module_name.as_ptr()) };
    if module == 0 {
        return None;
    }
    // SAFETY: `module` is a live module handle and `name` is null-terminated.
    let address = unsafe { GetProcAddress(module, name.as_ptr()) };
    (!address.is_null()).then_some(address)
}

/// Opts the process into per-monitor DPI awareness.
///
/// Without this the system scales the window's pixels, and a renderer that
/// draws its own antialiasing would be blurred by that scaling.
pub fn enable_dpi_awareness() {
    let Some(address) = user32_export(b"SetProcessDpiAwarenessContext\0") else {
        return;
    };
    // SAFETY: the resolved symbol has this documented signature. A failure
    // return is ignored because awareness may already be set by a manifest.
    unsafe {
        let set_context: unsafe extern "system" fn(isize) -> Bool = mem::transmute(address);
        set_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// Returns the scale factor to size a new window by, defaulting to 1.0.
fn primary_scale() -> f32 {
    let Some(address) = user32_export(b"GetDpiForSystem\0") else {
        return 1.0;
    };
    // SAFETY: the resolved symbol has this documented signature and takes no
    // arguments.
    let dpi = unsafe {
        let get_dpi: unsafe extern "system" fn() -> u32 = mem::transmute(address);
        get_dpi()
    };
    if dpi == 0 {
        1.0
    } else {
        dpi as f32 / USER_DEFAULT_SCREEN_DPI as f32
    }
}

fn ensure_window_class(instance: Hinstance, class_name: &[u16]) -> io::Result<()> {
    if WINDOW_CLASS_REGISTERED.get().is_some() {
        return Ok(());
    }
    // SAFETY: IDC_ARROW is a documented integer resource identifier, converted
    // to the pointer representation required by LoadCursorW.
    let cursor = unsafe { LoadCursorW(0, IDC_ARROW as *const u16) };
    let window_class = WndClassW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: 0,
        hCursor: cursor,
        // No background brush: the canvas covers the whole client area, so
        // letting the system erase first would only introduce a flash.
        hbrBackground: 0,
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    // SAFETY: window_class and class_name remain valid for the duration of the
    // call; window_proc matches the exact Win32 callback signature.
    if unsafe { RegisterClassW(&window_class) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        let _ = WINDOW_CLASS_REGISTERED.set(());
        Ok(())
    }
}

/// Grows a requested client size into the window size that contains it.
fn window_size_for_client(width: i32, height: i32) -> (i32, i32) {
    let mut rect = Rect::new(0, 0, width, height);
    // SAFETY: `rect` is writable stack storage and the style matches the one
    // the window is created with.
    let adjusted = unsafe { AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, 0) };
    if adjusted == 0 {
        (width, height)
    } else {
        (rect.width(), rect.height())
    }
}

fn create_window(
    instance: Hinstance,
    class_name: &[u16],
    definition: &WindowDefinition,
) -> io::Result<Hwnd> {
    let title = to_wide_null(&definition.title);
    let (width, height) = window_size_for_client(definition.width, definition.height);
    // SAFETY: class_name and title are null-terminated UTF-16 strings that stay
    // live through the call. All other handles are null because this is a top-
    // level window with no menu or creation data.
    let window = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            0,
            0,
            instance,
            ptr::null(),
        )
    };
    if window == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(window)
    }
}

/// Attaches the generated brand icon to a window.
fn apply_icons(window: Hwnd) {
    let (small, large) = ICONS.get_or_init(appicon::create);
    // SAFETY: the window belongs to this process and the icon handles, when
    // present, were created by CreateIconIndirect and outlive the process.
    unsafe {
        if let Some(small) = small {
            SendMessageW(window, WM_SETICON, ICON_SMALL, *small);
        }
        if let Some(large) = large {
            SendMessageW(window, WM_SETICON, ICON_BIG, *large);
        }
    }
}

fn show_and_update(window: Hwnd) {
    // SAFETY: window was returned by CreateWindowExW and is valid until the
    // message loop receives its destroy notification.
    unsafe {
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
    }
}

fn destroy_windows(windows: &[Hwnd]) {
    for &window in windows.iter().rev() {
        destroy_window(window);
    }
}

fn destroy_window(window: Hwnd) {
    // SAFETY: window belongs to this host process. Destruction synchronously
    // routes WM_DESTROY, which removes the matching registry entry.
    unsafe {
        DestroyWindow(window);
    }
}

fn message_loop() -> io::Result<()> {
    let mut message = Msg::default();
    loop {
        // SAFETY: message points to initialized writable storage and the other
        // arguments request messages for the current thread's full queue.
        let result = unsafe { GetMessageW(&mut message, 0, 0, 0) };
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        if result == 0 {
            return Ok(());
        }
        // SAFETY: GetMessageW populated message; these functions consume it
        // synchronously without retaining the pointer.
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn client_rect(window: Hwnd) -> Rect {
    let mut rect = Rect::default();
    // SAFETY: rect is writable stack storage for a synchronous query about a
    // window created by this process.
    unsafe {
        GetClientRect(window, &mut rect);
    }
    rect
}

fn invalidate(window: Hwnd) {
    // SAFETY: a null rectangle invalidates the whole client area; erase is
    // false because the canvas covers every pixel.
    unsafe {
        InvalidateRect(window, ptr::null(), 0);
    }
}

/// Invalidates one rectangle, so only that part is repainted and sent.
fn invalidate_region(window: Hwnd, region: CanvasRect) {
    let rect = Rect::new(
        region.left.floor() as i32,
        region.top.floor() as i32,
        region.right.ceil() as i32,
        region.bottom.ceil() as i32,
    );
    // SAFETY: rect is stack storage read synchronously; erase is false because
    // the canvas covers every pixel it will repaint.
    unsafe {
        InvalidateRect(window, &rect, 0);
    }
}

/// Extracts the signed client coordinates packed into an `LPARAM`.
fn mouse_position(lparam: Lparam) -> (i32, i32) {
    let raw = lparam as u32;
    ((raw & 0xFFFF) as i16 as i32, (raw >> 16) as i16 as i32)
}

fn wheel_delta(wparam: Wparam) -> i16 {
    ((wparam >> 16) as u16) as i16
}

/// Converts the portable closed state into its one documented User32 command.
///
/// Keeping this conversion separate makes the native boundary exhaustive and
/// unit-testable without creating a window. No caller can supply the integer.
const fn presentation_command(state: WindowState) -> i32 {
    match state {
        WindowState::Minimized => SW_MINIMIZE,
        WindowState::Maximized => SW_MAXIMIZE,
        WindowState::Restored => SW_RESTORE,
    }
}

/// Frame-cost guard for the animated surface.
///
/// The reveal is driven by a timer at [`REVEAL_INTERVAL_MILLIS`]. If a frame
/// costs more than that interval the animation drops frames, so the budget is
/// asserted rather than left to be noticed by eye. Only an optimised build is
/// measured: an unoptimised one is an order of magnitude slower, which is why
/// `start.bat` builds in release.
///
/// The measurement is wall clock, so it is only as isolated as the machine
/// running it. A single timed batch reports what the machine did during that
/// batch, which is not the same question as what the renderer costs: the same
/// commit has measured 8.5 ms per frame on an idle desktop and over 17 ms on a
/// loaded one. Every statistic here is therefore the **cheapest** of several
/// batches. Contention can only ever make a batch slower, so the cheapest one
/// is the closest observation to the renderer's own cost, and a rise in it is a
/// real rise in that cost rather than a busier machine. See
/// `docs/PERFORMANCE.md`.
#[cfg(all(test, not(debug_assertions)))]
mod frame_budget;
#[cfg(test)]
mod tests;
