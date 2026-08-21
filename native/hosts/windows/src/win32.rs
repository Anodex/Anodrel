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
mod crash;
mod document;
mod menu;
mod present;
mod product_tile;
mod registry;
mod startup_lab;
mod stats;
mod text;
mod ui_lab;
mod ui_session_view;

use std::{io, mem, ptr, sync::OnceLock, time::Instant};

use anodrel_canvas::{Canvas, Rect as CanvasRect, point};
use anodrel_core::SessionCloseSignal;
use anodrel_diagnostics::{Event, LogBook};
use anodrel_file_dialog::{FileDialogMailbox, FileDialogRequestKind, FileDialogSelection};
use anodrel_menu::MenuMailbox;
use anodrel_notifications::NotificationMailbox;
use anodrel_ui::UiDocument;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_instance::PrimaryInstance;

use anodrel_crash::{CrashSite, CrashSurface};
use anodrel_ui_session::UiFieldMailbox;
use anodrel_window::{WindowFocusMailbox, WindowState, WindowStateMailbox, WindowTitleMailbox};

use crate::product::PreflightOutcome;
use document::{Body, Document, Section};

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
    UiSession(ui_session_view::UiSessionView),
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
/// `launch` must come from the caller's verification-only preflight. The surface
/// never runs that check itself, so drawing code cannot decide that a launch is
/// possible. One outcome drives both the tile's availability and its diagnostic
/// entry, so the two can never disagree.
pub fn run_startup_lab(
    package: PackageFacts,
    instance: &PrimaryInstance,
    startup: std::time::Duration,
    launch: PreflightOutcome,
) -> io::Result<()> {
    let scale = primary_scale();
    let launch_available = launch.allows_launch();
    run_windows(
        vec![WindowDefinition {
            title: "Anodrel Startup Lab".to_owned(),
            width: (startup_lab::BASE_WIDTH * scale) as i32,
            height: (startup_lab::BASE_HEIGHT * scale) as i32,
            view: View::StartupLab(StartupLab {
                package,
                log: startup_log_book(launch.event()),
                startup_millis: startup.as_millis() as u64,
                working_set_bytes: stats::working_set_bytes(),
                last_frame_micros: 0,
                revealed_at: Instant::now(),
                hovered: None,
                ambient: false,
                launch_available,
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

/// Deliberate fault injection, for proving the containment path end to end.
///
/// A crash reporter that nobody can trigger is one nobody knows still works.
/// The `--crash-report-selftest` route proves the store can write; this proves
/// the part that matters more — that a panic raised inside a real window
/// message is contained, classified, recorded, and shut down cleanly instead of
/// aborting the process.
///
/// Compiled only in a debug build. `start.bat` and every shipped path build in
/// release, so this cannot exist in a binary a user runs.
#[cfg(debug_assertions)]
mod crash_selftest {
    use std::sync::atomic::{AtomicBool, Ordering};

    static ARMED: AtomicBool = AtomicBool::new(false);

    /// Arms the next window paint to panic.
    pub(super) fn arm() {
        ARMED.store(true, Ordering::Release);
    }

    /// Panics once if armed, disarming itself first.
    ///
    /// Disarming before panicking matters: the host repaints while shutting
    /// down, and a fault that re-armed itself would panic again inside the
    /// cleanup this route exists to observe.
    pub(super) fn fault_if_armed() {
        if ARMED.swap(false, Ordering::AcqRel) {
            panic!("deliberate fault injected by --crash-selftest-panic");
        }
    }
}

/// Opens the UI Lab and panics inside its first paint, then reports the result.
///
/// Available in debug builds only. What to look for afterwards: the process
/// exits without aborting, and a new record appears in the location named by
/// `docs/CRASH_REPORTS.md` with `site=window-procedure` and `surface=ui-lab` —
/// the surface being the proof that classification ran against a live window
/// rather than falling back to `unknown`.
#[cfg(debug_assertions)]
pub fn run_crash_selftest_panic() -> Result<(), Box<dyn std::error::Error>> {
    crash_selftest::arm();
    run_ui_lab()?;
    println!(
        "The injected fault was contained and the host shut down. \
         Check for a record with surface=ui-lab; see docs/CRASH_REPORTS.md."
    );
    Ok(())
}

/// Runs the real startup sequence, prints its readings, and exits.
///
/// The Startup Lab already shows a startup time and a working set in its
/// footer. This is the same measurement in a form a script can keep, so a
/// startup or memory figure can be recorded across builds instead of read off a
/// screenshot.
///
/// # What it measures, and what it does not
///
/// The elapsed time covers everything the host does before a surface could
/// open: package verification, the core health check, the private pipe
/// loopback, and the launch preflight. It stops there. **It does not include
/// creating the window or its first paint**, so it is a floor for cold start
/// and must never be quoted as time-to-first-frame. See `docs/PERFORMANCE.md`.
///
/// No window opens, so the memory readings are the host's own cost with nothing
/// rendered — also a floor, and the honest one to compare against another
/// runtime's idle process.
pub fn print_startup_report(application_id: &str, elapsed: std::time::Duration) {
    let memory = stats::memory_readings();
    // A local tooling format, shaped like the performance lab's report so both
    // can be retained the same way. It is not a protocol and has no reader here.
    println!(
        concat!(
            "{{\"benchmark\":\"anodrel.host.startup.v1\",",
            "\"startupMicroseconds\":{},",
            "\"workingSetBytes\":{},\"privateBytes\":{},",
            "\"applicationId\":\"{}\",",
            "\"scope\":\"host startup checks only; no window creation, no first paint\"}}"
        ),
        elapsed.as_micros(),
        memory.working_set_bytes,
        memory.private_bytes,
        application_id,
    );
}

/// Writes one crash record through the ordinary reporting path, then exits.
///
/// A crash record is only useful if it is actually written on the machine it is
/// meant to help, and the path that writes it is reached exactly once, during a
/// defect, when nobody is watching. This is how an operator confirms the
/// location, permissions, and format without waiting for a real failure.
///
/// It records the same `window-procedure` site a contained panic does, with no
/// window registered, so the surface is `unknown`. It opens no window and
/// prints only whether the record was written — never the location, which is in
/// `docs/CRASH_REPORTS.md` for a person to look up rather than for a process to
/// hand out.
pub fn run_crash_report_selftest() -> Result<(), Box<dyn std::error::Error>> {
    match crash::report(CrashSite::WindowProcedure, CrashSurface::Unknown) {
        Some(sequence) => {
            println!("Wrote crash record {sequence}. See docs/CRASH_REPORTS.md for its location.");
            Ok(())
        }
        None => Err(io::Error::other("the host could not write a crash record").into()),
    }
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

/// Opens one host-controlled native view that consumes exactly one authenticated
/// session's mailboxes. Actions enter only the bounded semantic-input mailbox
/// and remain incapable of native operations in this diagnostic.
#[allow(clippy::too_many_arguments)]
pub fn run_ui_session(
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    notifications: NotificationMailbox,
    menu: MenuMailbox,
    window_title: WindowTitleMailbox,
    window_state: WindowStateMailbox,
    window_focus: WindowFocusMailbox,
    display_name: &str,
    field_reads: UiFieldMailbox,
) -> io::Result<()> {
    run_authenticated_ui_session(
        "Anodrel UI Session Lab",
        mailbox,
        input_mailbox,
        close_signal,
        file_dialog_mailbox,
        file_text,
        notifications,
        menu,
        window_title,
        window_state,
        window_focus,
        display_name,
        field_reads,
    )
}

/// Opens one host-selected authenticated application session window.
///
/// The caller must supply resources created together for one already
/// authenticated session. This is host lifecycle code, not an application
/// window-management API: the application cannot create a window, pass a
/// handle, or attach a different session's resource.
///
/// `title` is the caption the host opens with. A session holding the
/// `window.title` grant may later propose a replacement, which the host
/// composes with `display_name` before applying — the application supplies one
/// half and never the other. See `docs/WINDOW_TITLE.md`.
///
/// `window_state` carries only minimise, maximise, and restore requests for
/// this same host-selected window. It is a closed command bridge, not a native
/// handle or a window-management API; see `docs/WINDOW_STATE.md`.
///
/// `window_focus` carries only a request to foreground this same host-selected
/// window. It exposes no target, input, retry, or observed focus state; see
/// `docs/WINDOW_FOCUS.md`.
#[allow(clippy::too_many_arguments)]
pub fn run_authenticated_ui_session(
    title: &str,
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    notifications: NotificationMailbox,
    menu: MenuMailbox,
    window_title: WindowTitleMailbox,
    window_state: WindowStateMailbox,
    window_focus: WindowFocusMailbox,
    display_name: &str,
    field_reads: UiFieldMailbox,
) -> io::Result<()> {
    let scale = primary_scale();
    run_windows(
        vec![WindowDefinition {
            title: title.to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiSession(
                ui_session_view::UiSessionView::new(
                    mailbox,
                    input_mailbox,
                    close_signal,
                    file_dialog_mailbox,
                    file_text,
                    notifications,
                )
                .with_menu(menu)
                .with_window_title(window_title, display_name)
                .with_window_state(window_state)
                .with_window_focus(window_focus)
                .with_field_reads(field_reads),
            ),
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
        let session_driven = matches!(definition.view, View::UiSession(_));
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
        if session_driven {
            // SAFETY: this window owns the mailbox consumer and stops this
            // low-frequency poll when the window is destroyed.
            unsafe {
                SetTimer(window, UI_SESSION_TIMER, UI_SESSION_POLL_INTERVAL_MILLIS, 0);
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
    let result = message_loop();
    // The loop normally ends only after the last window is destroyed, but a
    // contained panic ends it while views are still registered. Dropping them
    // here shuts down anything they own; the registry is a static and would
    // otherwise never be dropped at all.
    let _ = registry::clear();
    // No window can now collect a session that finished starting during
    // shutdown either, because a posted message is only delivered while the
    // loop runs.
    product_tile::discard();
    result
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

fn wheel_delta(wparam: Wparam) -> i16 {
    ((wparam >> 16) as u16) as i16
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

/// Opens the native window for one collected product session.
///
/// The window consumes only that session's grouped resources and owns its
/// lifetime: destroying it drops the session, which requests shutdown of the
/// verified child, the pipe worker, and the exit watcher.
fn open_product_session_window(
    session: anodrel_windows_product_session::RunningProductSession,
) -> io::Result<()> {
    let instance = module_handle()?;
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    ensure_window_class(instance, &class_name)?;
    let scale = primary_scale();
    let definition = WindowDefinition {
        // Named in full here, where there is room for it. The tile that opens
        // this window has to fit its label into a quarter of a strip.
        title: "Anodrel Development Product Fixture".to_owned(),
        width: (920.0 * scale) as i32,
        height: (660.0 * scale) as i32,
        view: View::UiSession(ui_session_view::UiSessionView::for_product_session(session)),
    };
    let window = create_window(instance, &class_name, &definition)?;
    if let Err(error) = registry::insert(window, definition.view) {
        destroy_window(window);
        return Err(error);
    }
    product_tile::note_window(window);
    apply_icons(window);
    // SAFETY: the window was just created on this thread's message queue and
    // its timer stops when the window is destroyed.
    unsafe {
        SetTimer(window, UI_SESSION_TIMER, UI_SESSION_POLL_INTERVAL_MILLIS, 0);
    }
    show_and_update(window);
    Ok(())
}

/// Maps a window's current layout into hierarchical accessibility elements.
///
/// The semantics come from the same layout the surface draws, so what a screen
/// reader is told cannot drift from what is on screen. A window with no UI
/// document publishes nothing, which is the honest answer for a document or
/// Startup Lab surface. It also carries copied host-owned focus and field values
/// alongside the same layout, while only an authenticated UI session adds the
/// bounded action sink used by an enabled button's Invoke pattern.
fn accessible_elements_for(window: Hwnd) -> anodrel_windows_uia::UiAutomationPublication {
    let rect = client_rect(window);
    let Ok(Some(publication)) =
        registry::accessibility_snapshot(window, rect.width() as f32, rect.height() as f32)
    else {
        return anodrel_windows_uia::UiAutomationPublication::empty();
    };
    anodrel_windows_uia::UiAutomationPublication::new(
        anodrel_windows_accessibility::accessible_elements(
            &publication.snapshot,
            client_origin(window),
        ),
        publication.field_values,
        publication.focused,
        publication.action_sink,
        publication
            .focus_route
            .map(|route| route.for_window(window, WM_ANODREL_UIA_FOCUS)),
    )
}

/// Raises one best-effort outbound focus notification after a real local move.
///
/// The publication is freshly derived after the view-registry mutation ended,
/// so the UI Automation adapter never observes a mutable view or a registry
/// lock. Its result is intentionally not logged or exposed to an application.
fn raise_accessibility_focus_changed(window: Hwnd) {
    anodrel_windows_uia::raise_focus_changed(window, accessible_elements_for(window));
}

/// Raises one best-effort subtree invalidation after accepted document replacement.
fn raise_accessibility_structure_changed(window: Hwnd) {
    anodrel_windows_uia::raise_structure_changed(window, accessible_elements_for(window));
}

/// Applies a pending host-only UI Automation focus request and repaints only
/// when a current validated target became focused.
fn service_accessibility_focus(window: Hwnd) {
    let rect = client_rect(window);
    let outcome =
        registry::service_accessibility_focus(window, rect.width() as f32, rect.height() as f32)
            .ok()
            .flatten();
    if outcome.is_some_and(|outcome| outcome.accepted && outcome.changed) {
        invalidate(window);
        raise_accessibility_focus_changed(window);
    }
}

/// Locates a window's client area on screen, with its current density.
fn client_origin(window: Hwnd) -> anodrel_windows_accessibility::ClientOrigin {
    let mut origin = Point { x: 0, y: 0 };
    // SAFETY: `origin` is writable stack storage and the window belongs to this
    // process; the call converts it in place to screen coordinates.
    unsafe {
        ClientToScreen(window, &mut origin);
    }
    // The layout is already composed at the display's real pixel density, so
    // its logical units are physical ones and need no further scaling.
    anodrel_windows_accessibility::ClientOrigin::new(origin.x, origin.y, 1.0)
}

/// Shows one pending notification for a session window, if it has one.
///
/// The Shell32 call runs outside the window registry's lock, so a slow shell
/// cannot block every other window's message handling. The notification-area
/// entry is created on first use and then reused, because creating one eagerly
/// would put an icon on screen for sessions that never notify.
fn service_notification(window: Hwnd) {
    let Ok(Some((request, entry))) = registry::take_notification_request(window) else {
        return;
    };

    let (entry, created) = match entry {
        Some(entry) => (Some(entry), None),
        // Host-owned brand artwork, the same icon the window already carries.
        // An application cannot supply, select, or replace it.
        None => match anodrel_windows_notifications::WindowsNotifications::create(
            window,
            ICONS.get_or_init(appicon::create).0.unwrap_or(0),
        ) {
            Ok(entry) => {
                let entry = std::sync::Arc::new(entry);
                (Some(std::sync::Arc::clone(&entry)), Some(entry))
            }
            Err(_) => (None, None),
        },
    };

    let shown = entry.is_some_and(|entry| {
        anodrel_notifications::NotificationService::show(entry.as_ref(), request.notification())
            .is_ok()
    });
    let _ = registry::complete_notification_request(window, request.id(), shown, created);
}

/// Answers one pending field read for a session window, if it has one.
///
/// Runs on the UI thread beside the other session bridges, because the values
/// belong to the window and a protocol worker never reaches into it. See
/// `docs/UI_FIELDS.md`.
fn service_field_read(window: Hwnd) {
    let Ok(Some(request_id)) = registry::take_field_read(window) else {
        return;
    };
    let _ = registry::complete_field_read(window, request_id);
}

/// Routes one typed character to whichever view this window carries.
///
/// Returns `None` when the window has no field-bearing view at all, so the
/// caller can fall through to the default procedure, and `Some(changed)` when a
/// view saw the character — including when it refused it, because a refusal is
/// still this window's answer rather than the system's.
fn type_character(window: Hwnd, rect: Rect, character: char) -> Option<bool> {
    let (width, height) = (rect.width() as f32, rect.height() as f32);
    registry::with_ui_lab(window, |lab| lab.type_character(width, height, character))
        .ok()
        .flatten()
        .or_else(|| {
            registry::with_ui_session(window, |session| {
                session.type_character(width, height, character)
            })
            .ok()
            .flatten()
        })
}

/// Routes one editing key the same way.
fn edit_focused_field(window: Hwnd, rect: Rect, edit: ui_lab::FieldEdit) -> Option<bool> {
    let (width, height) = (rect.width() as f32, rect.height() as f32);
    registry::with_ui_lab(window, |lab| lab.edit_focused_field(width, height, edit))
        .ok()
        .flatten()
        .or_else(|| {
            registry::with_ui_session(window, |session| {
                session.edit_focused_field(width, height, edit)
            })
            .ok()
            .flatten()
        })
}

/// Applies one pending window title for a session window, if it has one.
///
/// The `SetWindowTextW` call runs outside the window registry's lock, matching
/// how a notification is serviced: a caption change is fast, but nothing that
/// calls into User32 should hold a lock every other window's message handling
/// waits on.
///
/// The caption arriving here is already composed — the application's proposal
/// plus the session's validated display name — so this function has no way to
/// apply a title an application chose outright. See `docs/WINDOW_TITLE.md`.
fn service_window_title(window: Hwnd) {
    let Ok(Some((request_id, caption))) = registry::take_window_title_request(window) else {
        return;
    };
    let caption = to_wide_null(&caption);
    // SAFETY: this runs on the thread that created the window, and `caption` is
    // a null-terminated UTF-16 buffer that outlives the call.
    let applied = unsafe { SetWindowTextW(window, caption.as_ptr()) } != 0;
    let _ = registry::complete_window_title_request(window, request_id, applied);
}

/// Applies one pending closed presentation state for a session window.
///
/// `ShowWindow` returns the previous visibility, not an error indicator, so a
/// request that reaches this known UI-thread-owned window is complete after the
/// call. There is deliberately no state query before or after it: returning
/// either fact would turn a write-only command into window-state readback.
fn service_window_state(window: Hwnd) {
    let Ok(Some((request_id, state))) = registry::take_window_state_request(window) else {
        return;
    };
    let command = presentation_command(state);
    // SAFETY: this runs on the thread that created `window`, and `command` is
    // one of the three documented User32 presentation-state values above.
    unsafe { ShowWindow(window, command) };
    let _ = registry::complete_window_state_request(window, request_id, true);
}

/// Asks Windows to foreground this session window for one pending request.
///
/// This runs on the thread that created `window` and obtains the target only
/// from that window's registry entry. It does not observe the prior foreground
/// window, call `AllowSetForegroundWindow`, synthesize input, or retry a
/// refusal. Windows remains authoritative over foreground policy.
fn service_window_focus(window: Hwnd) {
    let Ok(Some(request_id)) = registry::take_window_focus_request(window) else {
        return;
    };
    // SAFETY: this runs on the thread that created `window`, and `window` is
    // resolved solely from that session's host-owned view registry entry.
    let requested = unsafe { SetForegroundWindow(window) } != 0;
    let _ = registry::complete_window_focus_request(window, request_id, requested);
}

/// Constructs and attaches one pending session menu on its owning UI thread.
///
/// Native construction happens before taking the process-wide view-registry
/// lock. The short locked attachment either replaces the current bar as one
/// operation or leaves it untouched; every failure becomes the portable
/// service's single safe unavailable outcome.
fn service_menu(window: Hwnd) {
    let Ok(Some(request)) = registry::take_menu_request(window) else {
        return;
    };
    let applied = menu::UnattachedMenu::build(&request)
        .and_then(|menu| registry::attach_menu(window, menu).ok().flatten())
        .unwrap_or(false);
    let _ = registry::complete_menu_request(window, request.id(), applied);
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

/// Starts one product session for the Startup Lab's launch tile.
///
/// The blocking verification and launch run on a worker; this returns
/// immediately so the message loop keeps pumping.
fn begin_product_session(window: Hwnd) {
    product_tile::request_start(move || {
        // SAFETY: posting to a window this process created is safe from any
        // thread, and the message carries no pointer or payload.
        let posted = unsafe { PostMessageW(window, WM_ANODREL_PRODUCT_SESSION, 0, 0) };
        if posted == 0 {
            // The surface closed while this session was starting, so nothing
            // will ever collect it. Ending it here is what stops a verified
            // child from outliving the host.
            product_tile::discard();
        }
    });
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
                                    stats::memory_readings().working_set_bytes as f32
                                        / (1024.0 * 1024.0)
                                ),
                            ),
                            (
                                // Reported beside the working set because the
                                // two answer different questions, and only this
                                // one adds up across a process tree.
                                "Private bytes".to_owned(),
                                format!(
                                    "{:.1} MB",
                                    stats::memory_readings().private_bytes as f32
                                        / (1024.0 * 1024.0)
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
        startup_lab::ActionKind::LaunchDevelopmentFixture => None,
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
        View::UiSession(session) => {
            let mut canvas = Canvas::new(width, height);
            ui_lab::draw(&mut canvas, session.lab());
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

/// Runs one window-procedure body, reporting `None` if it panicked.
///
/// A panic must not leave this function. `window_proc` is `extern "system"`,
/// which does not unwind, so Rust turns an escaping panic into an immediate
/// process abort — and an abort runs no destructor. That would leave a verified
/// product child running with no host, and a notification-area entry on screen
/// with nothing behind it.
///
/// The payload is dropped here rather than inspected. A panic message can carry
/// arbitrary values, and nothing derived from one may reach a protocol
/// response, the diagnostic ledger, a crash record, or an application.
fn contain_panic<R>(body: impl FnOnce() -> R) -> Option<R> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).ok()
}

unsafe extern "system" fn window_proc(
    window: Hwnd,
    message: Uint,
    wparam: Wparam,
    lparam: Lparam,
) -> Lresult {
    // SAFETY: the dispatch body keeps the same contract this callback always
    // had; only its unwinding behaviour changes.
    match contain_panic(|| unsafe { dispatch(window, message, wparam, lparam) }) {
        Some(result) => result,
        None => {
            // Leave evidence before leaving. Containment on its own makes a
            // defect look exactly like a clean exit, so the record is written
            // here rather than after the loop, while the window that was being
            // served is still known.
            crash::report_contained_panic(window);
            // Fail closed but orderly. Ending the message loop lets `run_windows`
            // return and drop every registered view, which shuts down a running
            // product child and removes any notification entry — the cleanup an
            // abort would have skipped entirely.
            // SAFETY: posting a quit message is valid from a window procedure.
            unsafe { PostQuitMessage(1) };
            0
        }
    }
}

unsafe fn dispatch(window: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam) -> Lresult {
    // Answered before the match below so an unrelated object request still
    // reaches the default procedure. It publishes semantics outward and only an
    // enabled authenticated-session button can offer its existing revision-bound
    // action mailbox. Focus reporting is a copied layout snapshot; neither
    // feature has a native-input or application callback route. Field values
    // are another copied snapshot and remain read-only to automation.
    if message == WM_GETOBJECT {
        let publication = accessible_elements_for(window);
        // SAFETY: this window belongs to the current thread's message queue,
        // which is the only thread that dispatches to this procedure.
        if let Some(result) =
            unsafe { anodrel_windows_uia::answer_get_object(window, wparam, lparam, publication) }
        {
            return result;
        }
    }
    if message == WM_ANODREL_UIA_FOCUS {
        service_accessibility_focus(window);
        return 0;
    }
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
            #[cfg(debug_assertions)]
            crash_selftest::fault_if_armed();
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
        WM_TIMER if wparam == UI_SESSION_TIMER => {
            if let Some((changed, close_requested)) =
                registry::poll_ui_session(window).ok().flatten()
            {
                if close_requested {
                    // SAFETY: the request is consumed only by this window's UI
                    // thread, which owns the host-created native handle.
                    unsafe { DestroyWindow(window) };
                    return 0;
                }
                if changed {
                    invalidate(window);
                    raise_accessibility_structure_changed(window);
                }
            }
            service_notification(window);
            service_menu(window);
            service_window_title(window);
            service_window_state(window);
            service_window_focus(window);
            service_field_read(window);
            if let Ok(Some(request)) = registry::take_file_dialog_request(window) {
                let selection = match request.kind() {
                    FileDialogRequestKind::Open => {
                        anodrel_windows_file_dialog::open_file_with_owner(window, request.filters())
                            .map(|path| {
                                path.map_or(
                                    FileDialogSelection::Cancelled,
                                    FileDialogSelection::Selected,
                                )
                            })
                    }
                    FileDialogRequestKind::Save => {
                        anodrel_windows_file_dialog::save_file_with_owner(window, request.filters())
                            .map(|path| {
                                path.map_or(
                                    FileDialogSelection::Cancelled,
                                    FileDialogSelection::Saved,
                                )
                            })
                    }
                    FileDialogRequestKind::OpenWithReference => {
                        let file_text = registry::file_text_service(window).ok().flatten();
                        match file_text {
                            Some(file_text) => {
                                anodrel_windows_file_dialog::open_file_with_owner_and_capture(
                                    window,
                                    request.filters(),
                                    |path| {
                                        let file =
                                            anodrel_windows_file_access::open_selected_file(path)
                                                .map_err(|_| ())?;
                                        file_text.register(file).map_err(|_| ())
                                    },
                                )
                                .map(|selection| {
                                    selection.map_or(
                                        FileDialogSelection::Cancelled,
                                        |(path, reference)| {
                                            FileDialogSelection::Captured(path, reference)
                                        },
                                    )
                                })
                            }
                            None => Err(anodrel_windows_file_dialog::FileDialogError::Unavailable),
                        }
                    }
                    FileDialogRequestKind::SaveWithReference => {
                        let file_text = registry::file_text_service(window).ok().flatten();
                        match file_text {
                            Some(file_text) => {
                                let file_write = file_text.write_service();
                                anodrel_windows_file_dialog::save_file_with_owner_and_capture(
                                    window,
                                    request.filters(),
                                    |path| {
                                        let file =
                                            anodrel_windows_file_access::open_save_file(path)
                                                .map_err(|_| ())?;
                                        file_write.register(file).map_err(|_| ())
                                    },
                                )
                                .map(|selection| {
                                    selection.map_or(
                                        FileDialogSelection::Cancelled,
                                        |(path, reference)| {
                                            FileDialogSelection::CapturedSave(path, reference)
                                        },
                                    )
                                })
                            }
                            None => Err(anodrel_windows_file_dialog::FileDialogError::Unavailable),
                        }
                    }
                };
                let _ = registry::complete_file_dialog_request(window, request.id(), selection);
            }
            0
        }
        WM_ACTIVATE => {
            set_ambient_running(window, (wparam & 0xFFFF) != WA_INACTIVE);
            0
        }
        WM_COMMAND => {
            let handled = registry::offer_menu_command(window, wparam, lparam)
                .ok()
                .flatten()
                .unwrap_or(false);
            if handled {
                // A current host-private normal menu command is now a bounded
                // semantic candidate. The application receives it only through
                // the authenticated pull path, never this native message.
                0
            } else {
                // SAFETY: unknown commands, accelerators, and controls retain
                // documented default Win32 handling unchanged.
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
        }
        WM_SETTINGCHANGE => {
            // Interactive native UI paints read the small direct Windows
            // appearance adapter. A system broadcast therefore schedules one
            // repaint without retaining settings or adding an application
            // observer/subscription surface.
            if registry::uses_system_appearance(window).unwrap_or(false) {
                invalidate(window);
                return 0;
            }
            // SAFETY: unhandled system settings messages retain standard
            // default Win32 processing for every other host view.
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        }
        WM_SIZE => {
            set_ambient_running(window, wparam != SIZE_MINIMIZED);
            let rect = client_rect(window);
            let _ = registry::with_ui_lab(window, |lab| {
                lab.clamp_scroll_offsets(rect.width() as f32, rect.height() as f32);
            });
            let _ = registry::with_ui_session(window, |session| {
                session.clamp_scroll_offsets(rect.width() as f32, rect.height() as f32);
            });
            0
        }
        WM_CHAR => {
            let rect = client_rect(window);
            // Backspace reaches a window as a control character rather than an
            // edit key, so it is separated here and routed as the edit it is.
            // Every other control character is dropped: a field refuses them,
            // and Tab and Enter are already handled as navigation.
            let handled = match u32::try_from(wparam).ok().and_then(char::from_u32) {
                Some(character) if u32::from(character) == CHAR_BACKSPACE => {
                    edit_focused_field(window, rect, ui_lab::FieldEdit::Backspace)
                }
                Some(character) if !character.is_control() => {
                    type_character(window, rect, character)
                }
                _ => None,
            };
            let Some(changed) = handled else {
                // SAFETY: a character this view does not consume is forwarded
                // unchanged to the documented default Win32 procedure.
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            };
            if changed {
                invalidate(window);
            }
            0
        }
        WM_KEYDOWN if matches!(wparam, VK_LEFT | VK_RIGHT | VK_HOME | VK_END | VK_DELETE) => {
            let rect = client_rect(window);
            let edit = match wparam {
                VK_LEFT => ui_lab::FieldEdit::Left,
                VK_RIGHT => ui_lab::FieldEdit::Right,
                VK_HOME => ui_lab::FieldEdit::Home,
                VK_END => ui_lab::FieldEdit::End,
                _ => ui_lab::FieldEdit::Delete,
            };
            let Some(changed) = edit_focused_field(window, rect, edit) else {
                // SAFETY: with no field focused these keys keep their default
                // meaning for the window.
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            };
            if changed {
                invalidate(window);
            }
            0
        }
        WM_KEYDOWN => {
            if !matches!(wparam, VK_TAB | VK_RETURN | VK_PRIOR | VK_NEXT) {
                // SAFETY: an unsupported key is forwarded unchanged to the
                // documented default Win32 procedure.
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            }
            let rect = client_rect(window);
            // SAFETY: querying one documented virtual-key state has no side
            // effect and returns a value owned by the current thread's input
            // state.
            let shift_down = unsafe { GetKeyState(VK_SHIFT) } < 0;
            if matches!(wparam, VK_PRIOR | VK_NEXT) {
                let changed = registry::with_ui_lab(window, |lab| {
                    lab.scroll_page(rect.width() as f32, rect.height() as f32, wparam == VK_NEXT)
                })
                .ok()
                .flatten()
                .or_else(|| {
                    registry::with_ui_session(window, |session| {
                        session.scroll_page(
                            rect.width() as f32,
                            rect.height() as f32,
                            wparam == VK_NEXT,
                        )
                    })
                    .ok()
                    .flatten()
                });
                let Some(changed) = changed else {
                    return unsafe { DefWindowProcW(window, message, wparam, lparam) };
                };
                if changed {
                    invalidate(window);
                }
                return 0;
            }
            let changed = registry::with_ui_lab(window, |lab| match wparam {
                VK_TAB if shift_down => {
                    lab.focus_previous(rect.width() as f32, rect.height() as f32)
                }
                VK_TAB => lab.focus_next(rect.width() as f32, rect.height() as f32),
                VK_RETURN => lab.activate_focused(rect.width() as f32, rect.height() as f32),
                _ => false,
            })
            .ok()
            .flatten()
            .or_else(|| {
                registry::with_ui_session(window, |session| match wparam {
                    VK_TAB if shift_down => {
                        session.focus_previous(rect.width() as f32, rect.height() as f32)
                    }
                    VK_TAB => session.focus_next(rect.width() as f32, rect.height() as f32),
                    VK_RETURN => {
                        session.activate_focused(rect.width() as f32, rect.height() as f32)
                    }
                    _ => false,
                })
                .ok()
                .flatten()
            });
            let Some(changed) = changed else {
                // Startup Lab and document views retain native default keyboard
                // behavior until their own input contracts exist.
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
            };
            if changed {
                invalidate(window);
                if wparam == VK_TAB {
                    raise_accessibility_focus_changed(window);
                }
            }
            0
        }
        WM_MOUSEWHEEL => {
            let delta = wheel_delta(wparam);
            if delta == 0 {
                return 0;
            }
            let rect = client_rect(window);
            let changed = registry::with_ui_lab(window, |lab| {
                lab.scroll_wheel_delta(rect.width() as f32, rect.height() as f32, i32::from(delta))
            })
            .ok()
            .flatten()
            .or_else(|| {
                registry::with_ui_session(window, |session| {
                    session.scroll_wheel_delta(
                        rect.width() as f32,
                        rect.height() as f32,
                        i32::from(delta),
                    )
                })
                .ok()
                .flatten()
            });
            if changed.unwrap_or(false) {
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
                registry::with_ui_session(window, |session| {
                    session.update_hover(
                        rect.width() as f32,
                        rect.height() as f32,
                        point(x as f32, y as f32),
                    )
                })
                .ok()
                .flatten()
                .unwrap_or_else(|| {
                    let hit = startup_lab::action_at(
                        rect.width() as f32,
                        rect.height() as f32,
                        point(x as f32, y as f32),
                    );
                    registry::with_startup_lab(window, |lab| {
                        // Hover follows the same availability value as drawing
                        // and clicking, so a planned tile never highlights.
                        let hovered = hit.filter(|index| {
                            startup_lab::tile_is_live(&startup_lab::ACTIONS[*index], lab)
                        });
                        let changed = lab.hovered != hovered;
                        lab.hovered = hovered;
                        changed
                    })
                    .ok()
                    .flatten()
                    .unwrap_or(false)
                })
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
                    registry::with_ui_session(window, |session| session.clear_hover())
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
                        })
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
                    registry::with_ui_session(window, |session| session.is_hovered())
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| {
                            registry::with_startup_lab(window, |lab| lab.hovered)
                                .ok()
                                .flatten()
                                .flatten()
                                .is_some()
                        })
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
            let (width, height) = (rect.width() as f32, rect.height() as f32);
            let at = point(x as f32, y as f32);
            // Focus first, then invoke. A click on a field only moves focus,
            // and a click on an action does both — pressing a control is also
            // how a person expects to focus it. Treating these as alternatives
            // is what left a field unreachable by pointer.
            if let Some((changed, focus_changed)) = registry::with_ui_lab(window, |lab| {
                let focused = lab.focus_at(width, height, at);
                (lab.invoke(width, height, at) || focused, focused)
            })
            .ok()
            .flatten()
            {
                if changed {
                    invalidate(window);
                }
                if focus_changed {
                    raise_accessibility_focus_changed(window);
                }
                return 0;
            }
            if let Some((changed, focus_changed)) = registry::with_ui_session(window, |session| {
                let focused = session.focus_at(width, height, at);
                (session.invoke(width, height, at) || focused, focused)
            })
            .ok()
            .flatten()
            {
                if changed {
                    invalidate(window);
                }
                if focus_changed {
                    raise_accessibility_focus_changed(window);
                }
                return 0;
            }
            let hit = startup_lab::action_at(
                rect.width() as f32,
                rect.height() as f32,
                point(x as f32, y as f32),
            )
            .map(|index| &startup_lab::ACTIONS[index]);
            if let Some(action) = hit
                && let Ok(Some(View::StartupLab(lab))) = registry::view_for(window)
                // Hit-testing and drawing read the same availability value, so a
                // tile drawn as planned cannot be activated by a click.
                && startup_lab::tile_is_live(action, &lab)
            {
                if action.kind == startup_lab::ActionKind::LaunchDevelopmentFixture {
                    begin_product_session(window);
                } else if let Some((title, document)) = action_document(action.kind, &lab) {
                    // A failure to open a diagnostic window is not fatal to the
                    // surface that launched it.
                    let _ = open_document_window(&title, document);
                }
            }
            0
        }
        WM_ANODREL_PRODUCT_SESSION => {
            match product_tile::take_started() {
                Some(session) => {
                    if open_product_session_window(session).is_err() {
                        // The session is dropped by the failed call, which
                        // requests its own shutdown. Release the guard so the
                        // tile can be tried again.
                        product_tile::release();
                    }
                }
                // A start that produced nothing has already released its guard
                // and reports no reason: a verified launch can fail for causes
                // this surface must not describe.
                None => product_tile::release(),
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
            // SAFETY: killing a timer for a window that has no session poll is
            // a no-op, and this window is being destroyed by the current UI
            // thread.
            unsafe { KillTimer(window, UI_SESSION_TIMER) };
            // Removing the view drops this window's product session, if it owns
            // one, which shuts down its child and joins both workers before the
            // guard is released. Shutdown precedes those joins, so this stays a
            // brief call rather than a wait on user-paced work.
            let removed = registry::remove(window);
            product_tile::note_destroyed(window);
            if removed.is_ok_and(|remaining| remaining == 0) {
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
        Body, Canvas, Instant, MIN_CLIENT_HEIGHT, MIN_CLIENT_WIDTH, PackageFacts, PreflightOutcome,
        StartupLab, action_document, document, mouse_position, presentation_command, startup_lab,
        startup_log_book, wheel_delta, window_size_for_client,
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
            log: startup_log_book(PreflightOutcome::NotLaunchable.event()),
            startup_millis: 1_240,
            working_set_bytes: 56 * 1024 * 1024,
            last_frame_micros: 3_180,
            revealed_at: Instant::now(),
            hovered: Some(2),
            ambient: false,
            // The shipped default: no machine record has been verified, so the
            // launch tile stays planned.
            launch_available: false,
        }
    }

    /// Representative surface state for a chosen preflight outcome.
    pub(super) fn startup_lab_fixture(launch_available: bool) -> StartupLab {
        StartupLab {
            launch_available,
            ..sample_lab()
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
    fn a_closed_presentation_state_maps_to_only_its_documented_user32_command() {
        assert_eq!(
            presentation_command(anodrel_window::WindowState::Minimized),
            super::SW_MINIMIZE
        );
        assert_eq!(
            presentation_command(anodrel_window::WindowState::Maximized),
            super::SW_MAXIMIZE
        );
        assert_eq!(
            presentation_command(anodrel_window::WindowState::Restored),
            super::SW_RESTORE
        );
    }

    #[test]
    fn mouse_coordinates_decode_as_signed_values() {
        assert_eq!(mouse_position(0x0010_0020), (32, 16));
        // A pointer dragged above or left of the client area reports negatives.
        let packed = (((-3_i16) as u16 as u32) << 16 | ((-7_i16) as u16 as u32)) as isize;
        assert_eq!(mouse_position(packed), (-7, -3));
    }

    #[test]
    fn wheel_deltas_decode_as_signed_high_words() {
        assert_eq!(wheel_delta(120usize << 16), 120);
        assert_eq!(wheel_delta(((-120i16 as u16) as usize) << 16), -120);
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
    fn a_panicking_window_message_is_contained_rather_than_aborting() {
        // `window_proc` is `extern "system"`, which does not unwind, so an
        // escaping panic aborts the process and runs no destructor. That would
        // strand a verified product child with no host to shut it down.
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let contained = super::contain_panic(|| panic!("a paint failure must not abort the host"));
        std::panic::set_hook(previous);

        assert_eq!(contained, None);
        assert_eq!(super::contain_panic(|| 42), Some(42));
    }

    #[test]
    fn a_crash_record_names_the_kind_of_surface_and_nothing_else() {
        use anodrel_crash::CrashSurface;

        let _exclusive = super::registry::tests_exclusive();
        // An unregistered window first: a panic can arrive before a view is
        // registered or after it is gone, and that must classify rather than
        // fail. The record path exists to leave evidence, so it has no branch
        // that gives up.
        assert_eq!(super::registry::crash_surface(-950), CrashSurface::Unknown);

        super::registry::insert(
            -951,
            super::View::Document(document::Document::from_text(
                "a title nothing may record",
                "test",
                "body",
            )),
        )
        .expect("view registers");
        super::registry::insert(-952, super::View::StartupLab(sample_lab()))
            .expect("view registers");

        assert_eq!(super::registry::crash_surface(-951), CrashSurface::Document);
        assert_eq!(
            super::registry::crash_surface(-952),
            CrashSurface::StartupLab
        );

        // The whole catalogue is plain labels. A surface that carried a title,
        // an application identity, or a handle would put unbounded text into a
        // file this platform promises holds none.
        for surface in CrashSurface::ALL {
            assert!(
                surface
                    .label()
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
            );
        }

        super::registry::clear().expect("registry clears");
    }

    #[test]
    fn clearing_the_registry_drops_views_the_loop_left_behind() {
        // A contained panic ends the message loop while windows are still
        // registered, so the host clears them itself.
        let _exclusive = super::registry::tests_exclusive();
        super::registry::insert(
            -901,
            super::View::Document(document::Document::from_text("stranded", "test", "body")),
        )
        .expect("view registers");

        assert_eq!(super::registry::clear().expect("registry clears"), 1);
        assert_eq!(super::registry::clear().expect("registry clears again"), 0);
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
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].0, "#0001");
        assert_eq!(events[4].0, "#0005");
        // The launch preflight sits between the transport check and the
        // surface being authorized, and reports only that the host ran it.
        assert!(events[3].1.contains("launch"));
        assert!(events[3].1.contains("Verified launch preflight completed."));
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
mod frame_budget {
    use super::{Canvas, REVEAL_INTERVAL_MILLIS, StartupLab, startup_lab, tests::sample_lab};

    /// Frames composed in one batch.
    ///
    /// The window covers the end of the mark's reveal, the transition to the
    /// settled ambient path, and the first settled frames — the most expensive
    /// stretch of the animation.
    const FRAMES: usize = 30;

    /// Milliseconds of animation between frames in a batch.
    const STEP_MILLIS: u64 = 10;

    /// Animation position the batch starts from.
    const START_MILLIS: u64 = 600;

    /// Batches timed, of which the cheapest observation is kept.
    ///
    /// Five is enough for one to land in a quiet slice of the scheduler without
    /// making the test slow: a batch costs well under half a second.
    const BATCHES: usize = 5;

    /// Composes one batch, returning each frame's cost in microseconds.
    fn batch(canvas: &mut Canvas, lab: &StartupLab) -> [f64; FRAMES] {
        let mut costs = [0.0; FRAMES];
        for (index, cost) in costs.iter_mut().enumerate() {
            let at = START_MILLIS + index as u64 * STEP_MILLIS;
            let started = std::time::Instant::now();
            startup_lab::draw(canvas, lab, at);
            *cost = started.elapsed().as_nanos() as f64 / 1_000.0;
        }
        costs
    }

    /// Returns the cheapest cost observed for each frame across [`BATCHES`].
    ///
    /// Frames are kept apart rather than averaged because the animation is not
    /// uniform: composing the mark's reveal costs several times what a settled
    /// frame costs, and a mean hides which frames are near the interval.
    fn cheapest_frames() -> [f64; FRAMES] {
        let lab = sample_lab();
        let mut canvas = Canvas::new(1_240, 900);
        // Warm the glyph and backdrop caches, as the first real frame does.
        startup_lab::draw(&mut canvas, &lab, START_MILLIS);

        let mut best = [f64::INFINITY; FRAMES];
        for _ in 0..BATCHES {
            for (kept, measured) in best.iter_mut().zip(batch(&mut canvas, &lab)) {
                *kept = kept.min(measured);
            }
        }
        best
    }

    /// The interval a frame has to fit inside, in microseconds.
    fn budget_micros() -> f64 {
        f64::from(REVEAL_INTERVAL_MILLIS) * 1_000.0
    }

    #[test]
    fn an_animated_frame_fits_inside_the_timer_interval() {
        let frames = cheapest_frames();
        let mean = frames.iter().sum::<f64>() / FRAMES as f64;
        let budget = budget_micros();
        // Reported on success as well as failure: a number that only appears
        // when the guard trips cannot show the trend that precedes it.
        println!("mean frame {mean:.0} us of a {budget:.0} us budget");
        assert!(
            mean < budget,
            "the cheapest of {BATCHES} batches still averages {mean:.0} us per frame, \
             over the {budget:.0} us budget"
        );
    }

    #[test]
    fn no_single_frame_of_the_reveal_overruns_the_interval() {
        let frames = cheapest_frames();
        let (index, worst) = frames
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .expect("a batch composes at least one frame");
        let at = START_MILLIS + index as u64 * STEP_MILLIS;
        let budget = budget_micros();
        println!("worst frame {worst:.0} us at {at} ms of a {budget:.0} us budget");
        // The mean can sit comfortably inside the interval while one frame
        // overruns it, and it is the single frame that drops, not the mean.
        assert!(
            *worst < budget,
            "the frame at {at} ms costs {worst:.0} us, over the {budget:.0} us budget"
        );
    }
}
