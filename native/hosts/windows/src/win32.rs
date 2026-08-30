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
//! [`dpi`] keeps optional DPI ABI resolution and process-module helpers away
//! from the window lifecycle and input-routing code.

#![allow(non_snake_case)]

mod appicon;
mod context_menu;
mod crash;
mod document;
mod dpi;
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
mod session_launch;
mod session_window_group;
mod size;
mod startup_lab;
mod startup_views;
mod stats;
mod text;
mod ui_lab;
mod ui_session_view;
mod uia_focus_event_probe;
mod uia_focus_probe;
mod uia_property_probe;
mod window_group_lab;

use std::{io, ptr, sync::OnceLock, time::Instant};

use anodrel_canvas::{Canvas, Rect as CanvasRect, point};
use anodrel_core::SessionCloseSignal;
use anodrel_diagnostics::LogBook;
use anodrel_file_dialog::{FileDialogMailbox, FileDialogRequestKind, FileDialogSelection};
use anodrel_menu::{ContextMenuMailbox, MenuMailbox};
use anodrel_notifications::NotificationMailbox;
use anodrel_ui::UiDocument;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox, UiWindowGroup, UiWindowId};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_folder_access::WindowsFolderEntryService;
use anodrel_windows_instance::PrimaryInstance;

use anodrel_crash::{CrashSite, CrashSurface};
use anodrel_ui_session::UiFieldMailbox;
use anodrel_window::{
    WindowFocusMailbox, WindowFullscreenMailbox, WindowFullscreenMode, WindowSizeMailbox,
    WindowStateChangesMailbox, WindowStateMailbox, WindowStateReadMailbox, WindowTitleMailbox,
    WindowTitleProposal,
};

use crate::product::PreflightOutcome;
use document::{Body, Document, Section};
use dpi::{module_handle, primary_scale, to_wide_null};
#[cfg(debug_assertions)]
use launch::crash_selftest;
use launch::run_windows;
use messages::{contain_panic, window_proc};
use services::*;
use startup_views::*;

pub use dpi::enable_dpi_awareness;
#[cfg(debug_assertions)]
pub use launch::run_crash_selftest_panic;
pub use launch::{
    print_startup_report, run, run_application, run_crash_report_selftest, run_grouped_ui_session,
    run_startup_lab, run_ui_lab, run_ui_preview, run_uia_property_probe, run_window_group_lab,
    run_window_lab,
};
pub use product_tile::FIXTURE_APPLICATION_ID;
pub(crate) use session_launch::run_ui_session_after_shown;
pub use session_launch::{run_authenticated_ui_session, run_ui_session};
pub use uia_focus_event_probe::run as run_uia_focus_event_probe;
pub use uia_focus_probe::run as run_uia_focus_probe;

mod raw;

use raw::*;

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

static ACTIVATION_MESSAGE: OnceLock<Uint> = OnceLock::new();
static WINDOW_CLASS_REGISTERED: OnceLock<()> = OnceLock::new();
static ICONS: OnceLock<(Option<isize>, Option<isize>)> = OnceLock::new();

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
