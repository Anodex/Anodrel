//! Direct Win32 window lifecycle and routing for Anodrel-owned surfaces.
//!
//! Raw window management remains here. GDI painting is split into small focused
//! modules so the startup lab can evolve without mixing visual layout into the
//! host's lifecycle code.

#![allow(non_snake_case)]

mod paint;
mod registry;
mod startup_lab;

use std::{io, ptr, sync::OnceLock};

use anodrel_windows_instance::PrimaryInstance;

type Atom = u16;
type Bool = i32;
type Dword = u32;
type Hbrush = isize;
type Hcursor = isize;
pub(super) type Hdc = isize;
type Hinstance = isize;
type Hwnd = isize;
type Lparam = isize;
type Lresult = isize;
type Uint = u32;
type Wparam = usize;

const CS_HREDRAW: Uint = 0x0002;
const CS_VREDRAW: Uint = 0x0001;
const WS_OVERLAPPEDWINDOW: Dword = 0x00CF_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
const SW_SHOW: i32 = 5;
const SW_RESTORE: i32 = 9;
const WM_DESTROY: Uint = 0x0002;
const WM_PAINT: Uint = 0x000F;
const IDC_ARROW: usize = 32_512;

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

#[derive(Clone)]
pub(super) struct StartupLab {
    pub(super) display_name: String,
    pub(super) application_id: String,
}

#[derive(Clone)]
enum View {
    Text(Vec<u16>),
    StartupLab(StartupLab),
}

struct WindowDefinition {
    title: String,
    width: i32,
    height: i32,
    view: View,
}

impl WindowDefinition {
    fn text(title: &str, text: &str, width: i32, height: i32) -> Self {
        Self {
            title: title.to_owned(),
            width,
            height,
            view: View::Text(to_wide_null(text)),
        }
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
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
}

static ACTIVATION_MESSAGE: OnceLock<Uint> = OnceLock::new();
static WINDOW_CLASS_REGISTERED: OnceLock<()> = OnceLock::new();

/// Opens the simple host-owned text surface.
pub fn run(title: &str, text: &str) -> io::Result<()> {
    run_windows(vec![WindowDefinition::text(title, text, 880, 520)], None)
}

/// Opens the validated application text surface as the primary host instance.
pub fn run_application(title: &str, text: &str, instance: &PrimaryInstance) -> io::Result<()> {
    run_windows(
        vec![WindowDefinition::text(title, text, 880, 520)],
        Some(instance),
    )
}

/// Opens the branded Startup Lab after the caller has completed its checks.
pub fn run_startup_lab(
    display_name: &str,
    application_id: &str,
    instance: &PrimaryInstance,
) -> io::Result<()> {
    run_windows(
        vec![WindowDefinition {
            title: "Anodrel Startup Lab".to_owned(),
            width: 1_160,
            height: 760,
            view: View::StartupLab(StartupLab {
                display_name: display_name.to_owned(),
                application_id: application_id.to_owned(),
            }),
        }],
        Some(instance),
    )
}

/// Opens two static host-owned windows to exercise the multi-window lifecycle.
pub fn run_window_lab() -> io::Result<()> {
    run_windows(
        vec![
            WindowDefinition::text(
                "Anodrel Window Lab - Primary",
                "Anodrel Window Lab\n\nThis primary window proves that the direct host can keep multiple host-owned windows alive on one User32 message loop.\n\nClose this window first: the companion window remains available. Close the final window to end the host process.",
                760,
                420,
            ),
            WindowDefinition::text(
                "Anodrel Window Lab - Companion",
                "Anodrel Window Lab\n\nThis companion has its own immutable host view. Its paint messages are routed by the real Win32 window handle, not a process-global surface.\n\nClose either window in any order. The host exits only after the last one closes.",
                760,
                420,
            ),
        ],
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
        let title = to_wide_null(&definition.title);
        let window = match create_window(
            instance,
            &class_name,
            &title,
            definition.width,
            definition.height,
        ) {
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

fn create_window(
    instance: Hinstance,
    class_name: &[u16],
    title: &[u16],
    width: i32,
    height: i32,
) -> io::Result<Hwnd> {
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
            // owned by the current process. Windows remains authoritative over
            // whether the foreground request is honored.
            unsafe {
                ShowWindow(window, SW_RESTORE);
                SetForegroundWindow(window);
            }
            0
        }
        WM_PAINT => {
            let mut paint = PaintStruct::default();
            // SAFETY: Windows calls this procedure for a valid window, and
            // paint is writable stack storage for the matching EndPaint call.
            let device_context = unsafe { BeginPaint(window, &mut paint) };
            if device_context != 0 {
                let mut rect = Rect::default();
                // SAFETY: rect is writable stack storage for the synchronous
                // client-area query, and the selected view remains immutable.
                unsafe {
                    GetClientRect(window, &mut rect);
                }
                if let Ok(Some(view)) = registry::view_for(window) {
                    match view {
                        View::Text(text) => paint::draw_text_surface(device_context, rect, &text),
                        View::StartupLab(lab) => startup_lab::draw(device_context, rect, &lab),
                    }
                }
                // SAFETY: BeginPaint initialized paint for this exact window.
                unsafe {
                    EndPaint(window, &paint);
                }
            }
            0
        }
        WM_DESTROY => {
            if registry::remove(window).is_ok_and(|remaining| remaining == 0) {
                // SAFETY: this only posts a quit message after the final
                // host-owned top-level window is being destroyed.
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
