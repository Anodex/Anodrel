//! Direct Win32 window lifecycle and routing for Anodrel surfaces.
//!
//! Raw window management stays here. Everything visible is composed by
//! [`anodrel_canvas`] into a single bitmap and presented in one blit, so this
//! module deals with messages and handles rather than with drawing.
//!
//! Submodules split that responsibility: [`present`] moves a canvas to the
//! screen, [`text`] turns GDI glyphs into canvas coverage, [`appicon`] builds
//! the window icon from brand geometry, and [`startup_lab`], [`document`], and
//! [`ui_lab`] own the host surfaces.

#![allow(non_snake_case)]

mod appicon;
mod document;
mod present;
mod registry;
mod startup_lab;
mod stats;
mod text;
mod ui_lab;

use std::{io, mem, ptr, sync::OnceLock, time::Instant};

use anodrel_canvas::{Canvas, Rect as CanvasRect, point};
use anodrel_diagnostics::{Event, LogBook};
use anodrel_ui::UiDocument;
use anodrel_windows_instance::PrimaryInstance;

use document::{Body, Document, Section};

type Atom = u16;
pub(super) type Bool = i32;
pub(super) type Dword = u32;
type Hbrush = isize;
type Hcursor = isize;
pub(super) type Hdc = isize;
type Hinstance = isize;
type Hwnd = isize;
type Lparam = isize;
type Lresult = isize;
pub(super) type Uint = u32;
type Wparam = usize;

const CS_HREDRAW: Uint = 0x0002;
const CS_VREDRAW: Uint = 0x0001;
const WS_OVERLAPPEDWINDOW: Dword = 0x00CF_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
const SW_SHOW: i32 = 5;
const SW_RESTORE: i32 = 9;
const WM_DESTROY: Uint = 0x0002;
const WM_PAINT: Uint = 0x000F;
const WM_ERASEBKGND: Uint = 0x0014;
const WM_GETMINMAXINFO: Uint = 0x0024;
const WM_SETICON: Uint = 0x0080;
const WM_SETCURSOR: Uint = 0x0020;
const WM_KEYDOWN: Uint = 0x0100;
const WM_MOUSEMOVE: Uint = 0x0200;
const WM_LBUTTONUP: Uint = 0x0202;
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
const VK_TAB: Wparam = 0x09;
const VK_RETURN: Wparam = 0x0D;

/// Timer driving the Startup Lab's reveal, at roughly 60 frames per second.
const REVEAL_TIMER: usize = 1;
const REVEAL_INTERVAL_MILLIS: Uint = 16;

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
}

/// Builds the diagnostic history displayed by the Startup Lab.
///
/// This function deliberately has no input: the displayed catalogue reflects
/// fixed host milestones rather than application text, operating-system errors,
/// paths, or arbitrary caller data.
fn startup_log_book() -> LogBook {
    let mut log = LogBook::new();
    for event in [
        Event::PackageVerified,
        Event::CoreHealthChecked,
        Event::PipeLoopbackChecked,
        Event::StartupLabAuthorized,
    ] {
        log.record(event)
            .expect("four fixed startup events fit in the diagnostic log");
    }
    log
}

#[derive(Clone)]
enum View {
    Document(Document),
    StartupLab(StartupLab),
    UiLab(ui_lab::UiLab),
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
    fn GetMessageW(message: *mut Msg, window: Hwnd, minimum: Uint, maximum: Uint) -> Bool;
    fn TranslateMessage(message: *const Msg) -> Bool;
    fn DispatchMessageW(message: *const Msg) -> Lresult;
    fn PostQuitMessage(exit_code: i32);
    fn BeginPaint(window: Hwnd, paint: *mut PaintStruct) -> Hdc;
    fn EndPaint(window: Hwnd, paint: *const PaintStruct) -> Bool;
    fn GetClientRect(window: Hwnd, rectangle: *mut Rect) -> Bool;
    fn LoadCursorW(instance: Hinstance, cursor_name: *const u16) -> Hcursor;
    fn SetCursor(cursor: Hcursor) -> Hcursor;
    fn GetKeyState(virtual_key: i32) -> i16;
    fn InvalidateRect(window: Hwnd, rectangle: *const Rect, erase: Bool) -> Bool;
    fn SendMessageW(window: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam) -> Lresult;
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

/// Opens the simple native document surface.
pub fn run(title: &str, text: &str) -> io::Result<()> {
    run_windows(
        vec![WindowDefinition::document(
            title,
            "Native surface",
            text,
            920,
            580,
        )],
        None,
    )
}

/// Opens the validated application text surface as the primary host instance.
pub fn run_application(
    title: &str,
    subtitle: &str,
    text: &str,
    instance: &PrimaryInstance,
) -> io::Result<()> {
    run_windows(
        vec![WindowDefinition::document(title, subtitle, text, 920, 580)],
        Some(instance),
    )
}

/// Opens the branded Startup Lab after the caller has completed its checks.
///
/// `startup` is the time from process start to this call, which the surface
/// reports as its startup reading.
pub fn run_startup_lab(
    package: PackageFacts,
    instance: &PrimaryInstance,
    startup: std::time::Duration,
) -> io::Result<()> {
    let scale = primary_scale();
    run_windows(
        vec![WindowDefinition {
            title: "Anodrel Startup Lab".to_owned(),
            width: (startup_lab::BASE_WIDTH * scale) as i32,
            height: (startup_lab::BASE_HEIGHT * scale) as i32,
            view: View::StartupLab(StartupLab {
                package,
                log: startup_log_book(),
                startup_millis: startup.as_millis() as u64,
                working_set_bytes: stats::working_set_bytes(),
                last_frame_micros: 0,
                revealed_at: Instant::now(),
                hovered: None,
                ambient: false,
            }),
        }],
        Some(instance),
    )
}

/// Opens two static native windows to exercise the multi-window lifecycle.
pub fn run_window_lab() -> io::Result<()> {
    run_windows(
        vec![
            WindowDefinition::document(
                "Anodrel Window Lab - Primary",
                "Multi-window lifecycle",
                "This primary window proves that the direct host can keep multiple native windows alive on one User32 message loop.\n\nClose this window first: the companion window remains available. Close the final window to end the host process.",
                760,
                460,
            ),
            WindowDefinition::document(
                "Anodrel Window Lab - Companion",
                "Multi-window lifecycle",
                "This companion has its own immutable host view. Its paint messages are routed by the real Win32 window handle, not a process-global surface.\n\nClose either window in any order. The host exits only after the last one closes.",
                760,
                460,
            ),
        ],
        None,
    )
}

/// Opens the host-owned visual and input test for the native UI foundation.
///
/// The screen uses a fixed document compiled into the host. Its action events
/// update only a visible diagnostic line; they never reach an application or a
/// native capability boundary.
pub fn run_ui_lab() -> io::Result<()> {
    let scale = primary_scale();
    run_windows(
        vec![WindowDefinition {
            title: "Anodrel UI Lab".to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiLab(ui_lab::UiLab::new()),
        }],
        None,
    )
}

/// Opens one operator-validated UI document as a local developer preview.
///
/// The caller must decode the document before this function is called. This
/// view has no package, protocol, application session, or capability binding;
/// its semantic actions remain local to the preview view.
pub fn run_ui_preview(document: UiDocument) -> io::Result<()> {
    let scale = primary_scale();
    run_windows(
        vec![WindowDefinition {
            title: "Anodrel UI Preview".to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiLab(ui_lab::UiLab::preview(document)),
        }],
        None,
    )
}

fn run_windows(
    definitions: Vec<WindowDefinition>,
    primary_instance: Option<&PrimaryInstance>,
) -> io::Result<()> {
    if definitions.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "host requires at least one window",
        ));
    }
    if let Some(primary_instance) = primary_instance {
        ACTIVATION_MESSAGE
            .set(primary_instance.activation_message())
            .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "activation message set"))?;
    }
    let instance = module_handle()?;
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    ensure_window_class(instance, &class_name)?;
    let mut windows = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let animated = matches!(definition.view, View::StartupLab(_));
        let window = match create_window(instance, &class_name, &definition) {
            Ok(window) => window,
            Err(error) => {
                destroy_windows(&windows);
                return Err(error);
            }
        };
        if let Err(error) = registry::insert(window, definition.view) {
            destroy_window(window);
            destroy_windows(&windows);
            return Err(error);
        }
        apply_icons(window);
        if animated {
            // SAFETY: the window was just created and belongs to this thread's
            // message queue. The timer is stopped when the reveal completes.
            unsafe {
                SetTimer(window, REVEAL_TIMER, REVEAL_INTERVAL_MILLIS, 0);
            }
        }
        windows.push(window);
    }
    if let Some(primary_instance) = primary_instance
        && let Err(error) = primary_instance.mark_ready()
    {
        destroy_windows(&windows);
        return Err(error);
    }
    for window in windows {
        show_and_update(window);
    }
    message_loop()
}

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

/// Opens an additional native window while the message loop is running.
fn open_document_window(title: &str, document: Document) -> io::Result<()> {
    let instance = module_handle()?;
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    ensure_window_class(instance, &class_name)?;
    let definition = WindowDefinition {
        title: title.to_owned(),
        width: 760,
        height: 560,
        view: View::Document(document),
    };
    let window = create_window(instance, &class_name, &definition)?;
    if let Err(error) = registry::insert(window, definition.view) {
        destroy_window(window);
        return Err(error);
    }
    apply_icons(window);
    show_and_update(window);
    Ok(())
}

/// Builds the document behind a linked action tile.
fn action_document(
    action: startup_lab::ActionKind,
    lab: &StartupLab,
) -> Option<(String, Document)> {
    match action {
        startup_lab::ActionKind::OpenLogs => Some((
            "Anodrel - Runtime Logs".to_owned(),
            Document {
                title: "Runtime Logs".to_owned(),
                subtitle: "Typed events for this process".to_owned(),
                body: Body::Sections(vec![
                    Section {
                        heading: "STARTUP EVENTS".to_owned(),
                        rows: lab
                            .log
                            .entries()
                            .map(|entry| {
                                (
                                    format!("#{:04}", entry.sequence()),
                                    format!(
                                        "{} | {} | {}",
                                        entry.level().label(),
                                        entry.component(),
                                        entry.message()
                                    ),
                                )
                            })
                            .collect(),
                    },
                    Section {
                        heading: "BOUNDARY".to_owned(),
                        rows: vec![
                            (
                                "Retention".to_owned(),
                                "64 in-memory events; oldest entries drop first".to_owned(),
                            ),
                            ("Application input".to_owned(), "not accepted".to_owned()),
                            (
                                "Persistence or export".to_owned(),
                                "not available".to_owned(),
                            ),
                        ],
                    },
                ]),
            },
        )),
        startup_lab::ActionKind::InspectPackage => Some((
            "Anodrel - Inspect Package".to_owned(),
            Document {
                title: "Inspect Package".to_owned(),
                subtitle: "Facts verified before this surface opened".to_owned(),
                body: Body::Sections(vec![
                    Section {
                        heading: "IDENTITY".to_owned(),
                        rows: vec![
                            ("Display name".to_owned(), lab.package.display_name.clone()),
                            (
                                "Application ID".to_owned(),
                                lab.package.application_id.clone(),
                            ),
                        ],
                    },
                    Section {
                        heading: "CONTENT".to_owned(),
                        rows: vec![
                            ("Format".to_owned(), lab.package.content_format.clone()),
                            ("Path".to_owned(), lab.package.content_path.clone()),
                            ("Bytes".to_owned(), lab.package.content_bytes.to_string()),
                            ("SHA-256".to_owned(), lab.package.content_digest.clone()),
                        ],
                    },
                    Section {
                        heading: "LIMITS".to_owned(),
                        rows: vec![
                            (
                                "Max manifest".to_owned(),
                                format!("{} bytes", anodrel_application::MAX_MANIFEST_BYTES),
                            ),
                            (
                                "Max content".to_owned(),
                                format!("{} bytes", anodrel_application::MAX_CONTENT_BYTES),
                            ),
                            (
                                "Publisher trust".to_owned(),
                                "not verified - see ROADMAP".to_owned(),
                            ),
                        ],
                    },
                ]),
            },
        )),
        startup_lab::ActionKind::RuntimeDiagnostics => Some((
            "Anodrel - Runtime Diagnostics".to_owned(),
            Document {
                title: "Runtime Diagnostics".to_owned(),
                subtitle: "Protocol, transport, and renderer state".to_owned(),
                body: Body::Sections(vec![
                    Section {
                        heading: "PROTOCOL".to_owned(),
                        rows: vec![
                            (
                                "Version".to_owned(),
                                format!(
                                    "{}.{}",
                                    anodrel_protocol::PROTOCOL_MAJOR,
                                    anodrel_protocol::PROTOCOL_MINOR
                                ),
                            ),
                            (
                                "Max request".to_owned(),
                                format!("{} bytes", anodrel_core::MAX_REQUEST_BYTES),
                            ),
                            (
                                "JSON depth".to_owned(),
                                anodrel_json::DEFAULT_MAX_DEPTH.to_string(),
                            ),
                        ],
                    },
                    Section {
                        heading: "TRANSPORT".to_owned(),
                        rows: vec![
                            (
                                "Frame magic".to_owned(),
                                String::from_utf8_lossy(&anodrel_wire::MAGIC).into_owned(),
                            ),
                            (
                                "Max payload".to_owned(),
                                format!("{} bytes", anodrel_wire::MAX_PAYLOAD_BYTES),
                            ),
                            (
                                "Frames per read".to_owned(),
                                anodrel_wire::MAX_FRAMES_PER_RECEIVE.to_string(),
                            ),
                            ("Pipe scope".to_owned(), "current logon session".to_owned()),
                        ],
                    },
                    Section {
                        heading: "PROCESS".to_owned(),
                        rows: vec![
                            (
                                "Working set".to_owned(),
                                format!(
                                    "{:.1} MB",
                                    stats::working_set_bytes() as f32 / (1024.0 * 1024.0)
                                ),
                            ),
                            ("Startup".to_owned(), format!("{} ms", lab.startup_millis)),
                            (
                                "Last frame".to_owned(),
                                format!("{:.2} ms", lab.last_frame_micros as f32 / 1000.0),
                            ),
                            (
                                "Runtime dependencies".to_owned(),
                                "0 third-party crates".to_owned(),
                            ),
                        ],
                    },
                ]),
            },
        )),
        startup_lab::ActionKind::LaunchSample => None,
    }
}

thread_local! {
    /// The animated surface, retained between paints.
    ///
    /// Repainting only a region requires the rest of the previous frame to
    /// still be there, so the animated window keeps its canvas rather than
    /// composing a fresh one each time. Document windows always redraw whole
    /// and need no such state.
    static SURFACE: std::cell::RefCell<Option<(Hwnd, Canvas)>> =
        const { std::cell::RefCell::new(None) };
}

/// Returns `true` when `inner` lies entirely within `outer`.
fn region_covers(outer: CanvasRect, inner: Rect) -> bool {
    (inner.left as f32) >= outer.left.floor()
        && (inner.top as f32) >= outer.top.floor()
        && (inner.right as f32) <= outer.right.ceil()
        && (inner.bottom as f32) <= outer.bottom.ceil()
}

/// Paints a window's view and presents it.
///
/// `update` is the rectangle Windows asked to be repainted. When it falls
/// inside the animated region and the surface has settled, only that region is
/// recomposed and sent; anything else redraws the whole surface.
///
/// Returns the time the frame took, which the Startup Lab reports on the next
/// frame.
fn paint(window: Hwnd, device_context: Hdc, view: &View, update: Rect) -> u64 {
    let started = Instant::now();
    let rect = client_rect(window);
    let width = rect.width().max(1) as u32;
    let height = rect.height().max(1) as u32;

    match view {
        View::Document(document) => {
            let mut canvas = Canvas::new(width, height);
            document::draw(&mut canvas, document);
            present::present(device_context, &canvas);
        }
        View::StartupLab(lab) => {
            let elapsed = lab.revealed_at.elapsed().as_millis() as u64;
            SURFACE.with(|cell| {
                let mut slot = cell.borrow_mut();
                let reusable = slot.as_ref().is_some_and(|(owner, canvas)| {
                    *owner == window && canvas.width() == width && canvas.height() == height
                });
                if !reusable {
                    // A new size invalidates every cached layer, including the
                    // backdrop and the pre-composed hero.
                    startup_lab::invalidate_caches();
                    *slot = Some((window, Canvas::new(width, height)));
                }
                let Some((_, canvas)) = slot.as_mut() else {
                    return;
                };

                let region = (elapsed >= startup_lab::REVEAL_MILLIS)
                    .then(|| startup_lab::ambient_region(width as f32, height as f32))
                    .flatten()
                    .filter(|region| reusable && region_covers(*region, update));

                match region {
                    Some(region) if startup_lab::draw_ambient(canvas, lab, elapsed) => {
                        present::present_region(
                            device_context,
                            canvas,
                            region.left.floor().max(0.0) as u32,
                            region.top.floor().max(0.0) as u32,
                            region.width().ceil() as u32,
                            region.height().ceil() as u32,
                        );
                    }
                    _ => {
                        startup_lab::draw(canvas, lab, elapsed);
                        present::present(device_context, canvas);
                    }
                }
            });
        }
        View::UiLab(lab) => {
            let mut canvas = Canvas::new(width, height);
            ui_lab::draw(&mut canvas, lab);
            present::present(device_context, &canvas);
        }
    }
    started.elapsed().as_micros() as u64
}

/// Starts or stops ambient motion for a settled surface.
///
/// Motion is suspended whenever the window cannot be seen or is not being
/// looked at. A background window must cost nothing.
fn set_ambient_running(window: Hwnd, running: bool) {
    let settled = registry::with_startup_lab(window, |lab| lab.ambient)
        .ok()
        .flatten()
        .unwrap_or(false);
    if !settled {
        return;
    }
    // SAFETY: the window belongs to this process; setting a timer that already
    // exists resets it, and killing one that does not is a no-op.
    unsafe {
        if running {
            SetTimer(window, REVEAL_TIMER, AMBIENT_INTERVAL_MILLIS, 0);
        } else {
            KillTimer(window, REVEAL_TIMER);
        }
    }
}

unsafe extern "system" fn window_proc(
    window: Hwnd,
    message: Uint,
    wparam: Wparam,
    lparam: Lparam,
) -> Lresult {
    match message {
        message
            if ACTIVATION_MESSAGE
                .get()
                .is_some_and(|expected| *expected == message) =>
        {
            // SAFETY: the activation message carries no data and this window is
            // created by the current process. Windows remains authoritative over
            // whether the foreground request is honored.
            unsafe {
                ShowWindow(window, SW_RESTORE);
                SetForegroundWindow(window);
            }
            0
        }
        // The canvas covers every pixel, so erasing first would only flash.
        WM_ERASEBKGND => 1,
        WM_GETMINMAXINFO => {
            let (width, height) = window_size_for_client(MIN_CLIENT_WIDTH, MIN_CLIENT_HEIGHT);
            // SAFETY: for this message lparam points to a writable MINMAXINFO
            // supplied by the system.
            unsafe {
                let info = &mut *(lparam as *mut MinMaxInfo);
                info.minTrackSize = Point {
                    x: width,
                    y: height,
                };
            }
            0
        }
        WM_PAINT => {
            let mut paint_struct = PaintStruct::default();
            // SAFETY: Windows calls this procedure for a valid window, and
            // paint_struct is writable stack storage for the matching EndPaint.
            let device_context = unsafe { BeginPaint(window, &mut paint_struct) };
            if device_context != 0 {
                if let Ok(Some(view)) = registry::view_for(window) {
                    let micros = paint(window, device_context, &view, paint_struct.rcPaint);
                    let _ = registry::with_startup_lab(window, |lab| {
                        lab.last_frame_micros = micros;
                    });
                }
                // SAFETY: BeginPaint initialized paint_struct for this window.
                unsafe {
                    EndPaint(window, &paint_struct);
                }
            }
            0
        }
        WM_TIMER if wparam == REVEAL_TIMER => {
            let state = registry::with_startup_lab(window, |lab| {
                let elapsed = lab.revealed_at.elapsed().as_millis() as u64;
                let settling = !lab.ambient && elapsed >= startup_lab::REVEAL_MILLIS;
                if settling {
                    lab.ambient = true;
                }
                (elapsed, lab.ambient, settling)
            })
            .ok()
            .flatten();
            let Some((_, ambient, settling)) = state else {
                // Not an animated surface any more; stop waking for it.
                // SAFETY: killing a timer that does not exist is a no-op.
                unsafe { KillTimer(window, REVEAL_TIMER) };
                return 0;
            };

            if settling {
                // The reveal is done. Drop from the reveal's frame rate to the
                // ambient one rather than stopping: the mark keeps breathing,
                // but at a cadence a settled screen can afford.
                // SAFETY: re-setting an existing timer changes its interval.
                unsafe {
                    SetTimer(window, REVEAL_TIMER, AMBIENT_INTERVAL_MILLIS, 0);
                }
            }

            let rect = client_rect(window);
            match ambient
                .then(|| startup_lab::ambient_region(rect.width() as f32, rect.height() as f32))
                .flatten()
            {
                // The mark moves, and its translucent foreground detail band
                // is redrawn above it. Both fit inside this bounded region.
                Some(region) => invalidate_region(window, region),
                None => invalidate(window),
            }
            0
        }
        WM_ACTIVATE => {
            set_ambient_running(window, (wparam & 0xFFFF) != WA_INACTIVE);
            0
        }
        WM_SIZE => {
            set_ambient_running(window, wparam != SIZE_MINIMIZED);
            0
        }
        WM_KEYDOWN => {
            if !matches!(wparam, VK_TAB | VK_RETURN) {
                // SAFETY: an unsupported key is forwarded unchanged to the
                // documented default Win32 procedure.
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            }
            let rect = client_rect(window);
            // SAFETY: querying one documented virtual-key state has no side
            // effect and returns a value owned by the current thread's input
            // state.
            let shift_down = unsafe { GetKeyState(VK_SHIFT) } < 0;
            let Some(changed) = registry::with_ui_lab(window, |lab| match wparam {
                VK_TAB if shift_down => {
                    lab.focus_previous(rect.width() as f32, rect.height() as f32)
                }
                VK_TAB => lab.focus_next(rect.width() as f32, rect.height() as f32),
                VK_RETURN => lab.activate_focused(rect.width() as f32, rect.height() as f32),
                _ => false,
            })
            .ok()
            .flatten() else {
                // The only host keyboard adapter is deliberately UI Lab
                // specific. Startup Lab and document views retain the native
                // default behavior until their own input contracts exist.
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            };
            if changed {
                invalidate(window);
            }
            0
        }
        WM_MOUSEMOVE => {
            let (x, y) = mouse_position(lparam);
            let rect = client_rect(window);
            let changed = registry::with_ui_lab(window, |lab| {
                lab.update_hover(
                    rect.width() as f32,
                    rect.height() as f32,
                    point(x as f32, y as f32),
                )
            })
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                let hovered = startup_lab::action_at(
                    rect.width() as f32,
                    rect.height() as f32,
                    point(x as f32, y as f32),
                )
                .filter(|index| startup_lab::ACTIONS[*index].linked);
                registry::with_startup_lab(window, |lab| {
                    let changed = lab.hovered != hovered;
                    lab.hovered = hovered;
                    changed
                })
                .ok()
                .flatten()
                .unwrap_or(false)
            });
            if changed {
                invalidate(window);
            }
            let mut track = TrackMouseEventStruct {
                cbSize: mem::size_of::<TrackMouseEventStruct>() as Dword,
                dwFlags: TME_LEAVE,
                hwndTrack: window,
                dwHoverTime: 0,
            };
            // SAFETY: `track` is writable stack storage whose declared size
            // matches the struct, and the window belongs to this process.
            unsafe {
                TrackMouseEvent(&mut track);
            }
            0
        }
        WM_MOUSELEAVE => {
            let changed = registry::with_ui_lab(window, ui_lab::UiLab::clear_hover)
                .ok()
                .flatten()
                .unwrap_or_else(|| {
                    registry::with_startup_lab(window, |lab| {
                        let changed = lab.hovered.is_some();
                        lab.hovered = None;
                        changed
                    })
                    .ok()
                    .flatten()
                    .unwrap_or(false)
                });
            if changed {
                invalidate(window);
            }
            0
        }
        WM_SETCURSOR if (lparam as u32 & 0xFFFF) as isize == HTCLIENT => {
            let hovered = registry::with_ui_lab(window, |lab| lab.hovered.is_some())
                .ok()
                .flatten()
                .unwrap_or_else(|| {
                    registry::with_startup_lab(window, |lab| lab.hovered)
                        .ok()
                        .flatten()
                        .flatten()
                        .is_some()
                });
            let cursor_id = if hovered { IDC_HAND } else { IDC_ARROW };
            // SAFETY: both identifiers are documented integer resources, and
            // LoadCursorW returns a shared cursor that must not be destroyed.
            unsafe {
                SetCursor(LoadCursorW(0, cursor_id as *const u16));
            }
            1
        }
        WM_LBUTTONUP => {
            let (x, y) = mouse_position(lparam);
            let rect = client_rect(window);
            if let Some(changed) = registry::with_ui_lab(window, |lab| {
                lab.invoke(
                    rect.width() as f32,
                    rect.height() as f32,
                    point(x as f32, y as f32),
                )
            })
            .ok()
            .flatten()
            {
                if changed {
                    invalidate(window);
                }
                return 0;
            }
            let clicked = startup_lab::action_at(
                rect.width() as f32,
                rect.height() as f32,
                point(x as f32, y as f32),
            )
            .map(|index| &startup_lab::ACTIONS[index])
            .filter(|action| action.linked);
            if let Some(action) = clicked
                && let Ok(Some(View::StartupLab(lab))) = registry::view_for(window)
                && let Some((title, document)) = action_document(action.kind, &lab)
            {
                // A failure to open a diagnostic window is not fatal to the
                // surface that launched it.
                let _ = open_document_window(&title, document);
            }
            0
        }
        WM_DPICHANGED => {
            // SAFETY: for this message lparam points to a RECT the system has
            // sized for the new DPI.
            let suggested = unsafe { *(lparam as *const Rect) };
            // SAFETY: the window belongs to this process; z-order and
            // activation are left untouched.
            unsafe {
                SetWindowPos(
                    window,
                    0,
                    suggested.left,
                    suggested.top,
                    suggested.width(),
                    suggested.height(),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            invalidate(window);
            0
        }
        WM_DESTROY => {
            if registry::remove(window).is_ok_and(|remaining| remaining == 0) {
                // SAFETY: this only posts a quit message after the final
                // native top-level window is being destroyed.
                unsafe { PostQuitMessage(0) };
            }
            0
        }
        _ => {
            // SAFETY: all unhandled messages are forwarded unchanged to the
            // documented default Win32 procedure.
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Body, Canvas, Instant, MIN_CLIENT_HEIGHT, MIN_CLIENT_WIDTH, PackageFacts, StartupLab,
        action_document, document, mouse_position, startup_lab, startup_log_book,
        window_size_for_client,
    };
    /// Representative surface state, matching the shipped sample package.
    pub(super) fn sample_lab() -> StartupLab {
        StartupLab {
            package: PackageFacts {
                display_name: "Anodrel Sample".to_owned(),
                application_id: "org.anodrel.sample".to_owned(),
                content_format: "anodrel.text.v1".to_owned(),
                content_path: "content/main.txt".to_owned(),
                content_digest: "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"
                    .to_owned(),
                content_bytes: 214,
            },
            log: startup_log_book(),
            startup_millis: 1_240,
            working_set_bytes: 56 * 1024 * 1024,
            last_frame_micros: 3_180,
            revealed_at: Instant::now(),
            hovered: Some(2),
            ambient: false,
        }
    }

    /// Counts pixels brighter than `threshold` in summed channels.
    ///
    /// A plain "differs from the backdrop" count is useless here: the hero's
    /// radial bloom tints nearly every pixel from the first frame. Content —
    /// the mark, type, cards — is far brighter than that wash, so a luminance
    /// threshold is what actually distinguishes drawn content from background.
    fn lit_pixels(canvas: &Canvas, threshold: u16) -> usize {
        (0..canvas.height())
            .flat_map(|y| (0..canvas.width()).map(move |x| (x as i32, y as i32)))
            .filter(|(x, y)| {
                let color = canvas.pixel(*x, *y);
                u16::from(color.red) + u16::from(color.green) + u16::from(color.blue) > threshold
            })
            .count()
    }

    #[test]
    fn a_client_size_grows_into_a_larger_window_size() {
        let (width, height) = window_size_for_client(MIN_CLIENT_WIDTH, MIN_CLIENT_HEIGHT);
        assert!(width >= MIN_CLIENT_WIDTH);
        assert!(height > MIN_CLIENT_HEIGHT, "a title bar should add height");
    }

    #[test]
    fn mouse_coordinates_decode_as_signed_values() {
        assert_eq!(mouse_position(0x0010_0020), (32, 16));
        // A pointer dragged above or left of the client area reports negatives.
        let packed = (((-3_i16) as u16 as u32) << 16 | ((-7_i16) as u16 as u32)) as isize;
        assert_eq!(mouse_position(packed), (-7, -3));
    }

    #[test]
    fn the_startup_lab_composes_at_every_supported_client_size() {
        let lab = sample_lab();
        for (width, height) in [
            (MIN_CLIENT_WIDTH as u32, MIN_CLIENT_HEIGHT as u32),
            (1_240, 900),
            (2_480, 1_800),
        ] {
            let mut canvas = Canvas::new(width, height);
            startup_lab::draw(&mut canvas, &lab, 99_999);
            let painted = lit_pixels(&canvas, 150);
            assert!(
                painted > (width as usize * height as usize) / 200,
                "{width}x{height} composed only {painted} lit pixels"
            );
        }
    }

    fn frame_at(elapsed: u64) -> Canvas {
        let mut canvas = Canvas::new(1_240, 900);
        startup_lab::draw(&mut canvas, &sample_lab(), elapsed);
        canvas
    }

    fn differing_pixels(left: &Canvas, right: &Canvas) -> usize {
        (0..left.height())
            .flat_map(|y| (0..left.width()).map(move |x| (x as i32, y as i32)))
            .filter(|(x, y)| left.pixel(*x, *y) != right.pixel(*x, *y))
            .count()
    }

    #[test]
    fn the_reveal_adds_content_over_time() {
        let opening = lit_pixels(&frame_at(0), 150);
        let midway = lit_pixels(&frame_at(startup_lab::REVEAL_MILLIS / 2), 150);
        let settled = lit_pixels(&frame_at(startup_lab::REVEAL_MILLIS), 150);
        assert!(opening < midway, "the reveal should add content over time");
        assert!(
            midway < settled,
            "the reveal should finish fuller than midway"
        );
    }

    #[test]
    fn everything_but_the_ambient_loop_is_static_once_revealed() {
        // Sampling a whole ambient cycle apart puts the animation at the same
        // phase, so any difference would be a reveal stage still running.
        let settled = frame_at(startup_lab::REVEAL_MILLIS);
        let later = frame_at(startup_lab::REVEAL_MILLIS + startup_lab::AMBIENT_CYCLE_MILLIS);
        assert_eq!(
            differing_pixels(&settled, &later),
            0,
            "the surface must be identical at equal ambient phase"
        );
    }

    #[test]
    fn ambient_motion_actually_moves() {
        // Mid-sweep against a point in the cycle with no sweep at all.
        let swept = frame_at(startup_lab::REVEAL_MILLIS + startup_lab::AMBIENT_CYCLE_MILLIS / 10);
        let quiet = frame_at(startup_lab::REVEAL_MILLIS + startup_lab::AMBIENT_CYCLE_MILLIS / 2);
        assert!(
            differing_pixels(&swept, &quiet) > 5_000,
            "the mark should visibly change across the ambient cycle"
        );
    }

    #[test]
    fn a_partial_ambient_frame_reproduces_a_full_one() {
        // The partial path restores the backdrop and recomposites cached
        // layers. If it ever diverged from a full compose, the mark's region
        // would drift out of step with the rest of the surface.
        let elapsed = startup_lab::REVEAL_MILLIS + 900;
        let full = frame_at(elapsed);
        let mut partial = frame_at(elapsed);
        assert!(
            startup_lab::draw_ambient(&mut partial, &sample_lab(), elapsed),
            "the ambient path should be available once settled"
        );
        assert_eq!(
            differing_pixels(&full, &partial),
            0,
            "a partial update must match a full compose exactly"
        );
    }

    #[test]
    fn ambient_motion_stays_inside_its_declared_region() {
        // Whatever moves must be inside the region the host invalidates, or
        // the screen would tear where an update was never sent.
        let region = startup_lab::ambient_region(1_240.0, 900.0).expect("region available");
        let swept = frame_at(startup_lab::REVEAL_MILLIS + startup_lab::AMBIENT_CYCLE_MILLIS / 10);
        let quiet = frame_at(startup_lab::REVEAL_MILLIS + startup_lab::AMBIENT_CYCLE_MILLIS / 2);
        for y in 0..900_i32 {
            for x in 0..1_240_i32 {
                if swept.pixel(x, y) == quiet.pixel(x, y) {
                    continue;
                }
                assert!(
                    (x as f32) >= region.left.floor()
                        && (x as f32) < region.right.ceil()
                        && (y as f32) >= region.top.floor()
                        && (y as f32) < region.bottom.ceil(),
                    "pixel ({x}, {y}) changes outside the ambient region {region:?}"
                );
            }
        }
    }

    #[test]
    fn linked_actions_produce_a_document_and_planned_actions_do_not() {
        let lab = sample_lab();
        for action in &startup_lab::ACTIONS {
            let produced = action_document(action.kind, &lab);
            assert_eq!(
                produced.is_some(),
                action.linked,
                "{:?} disagrees with its linked state",
                action.kind
            );
        }
    }

    #[test]
    fn log_document_contains_only_the_fixed_startup_catalogue() {
        let lab = sample_lab();
        let Some((_, document)) = action_document(startup_lab::ActionKind::OpenLogs, &lab) else {
            panic!("the linked log action needs its host document");
        };
        let Body::Sections(sections) = document.body else {
            panic!("the log document must be structured");
        };
        let events = &sections[0].rows;
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].0, "#0001");
        assert_eq!(events[3].0, "#0004");
        for (_, reading) in events {
            assert!(!reading.contains(char::from(92)));
            assert!(!reading.contains('/'));
            assert!(!reading.contains(':'));
        }
    }

    #[test]
    fn an_action_document_never_carries_a_filesystem_path() {
        let lab = sample_lab();
        for action in &startup_lab::ACTIONS {
            let Some((_, document)) = action_document(action.kind, &lab) else {
                continue;
            };
            let mut canvas = Canvas::new(760, 560);
            document::draw(&mut canvas, &document);
            assert!(lit_pixels(&canvas, 150) > 1_000, "document drew nothing");
        }
        // The window layer only ever holds the manifest-relative path.
        assert!(!lab.package.content_path.contains(':'));
        assert!(!lab.package.content_path.contains('\\'));
    }
}

/// Frame-cost guard for the animated surface.
///
/// The reveal is driven by a timer at [`REVEAL_INTERVAL_MILLIS`]. If a frame
/// costs more than that interval the animation drops frames, so the budget is
/// asserted rather than left to be noticed by eye. Only an optimised build is
/// measured: an unoptimised one is an order of magnitude slower, which is why
/// `start.bat` builds in release.
#[cfg(all(test, not(debug_assertions)))]
mod frame_budget {
    use super::{Canvas, REVEAL_INTERVAL_MILLIS, startup_lab, tests::sample_lab};

    #[test]
    fn an_animated_frame_fits_inside_the_timer_interval() {
        let lab = sample_lab();
        let mut canvas = Canvas::new(1_240, 900);
        // Warm the glyph and backdrop caches, as the first real frame does.
        startup_lab::draw(&mut canvas, &lab, 600);

        let frames = 30_u32;
        let started = std::time::Instant::now();
        for index in 0..frames {
            startup_lab::draw(&mut canvas, &lab, 600 + u64::from(index) * 10);
        }
        let per_frame_micros = started.elapsed().as_micros() as f64 / f64::from(frames);
        let budget_micros = f64::from(REVEAL_INTERVAL_MILLIS) * 1_000.0;
        assert!(
            per_frame_micros < budget_micros,
            "a frame costs {per_frame_micros:.0} us, over the {budget_micros:.0} us budget"
        );
    }
}
