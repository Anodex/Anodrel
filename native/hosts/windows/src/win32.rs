//! Direct Win32 window lifecycle and routing for Anodrel-owned surfaces.
//!
//! Raw window management remains here. GDI painting is split into small focused
//! modules so the startup lab can evolve without mixing visual layout into the
//! host's lifecycle code.

#![allow(non_snake_case)]

mod paint;
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

pub(super) struct StartupLab {
    pub(super) display_name: String,
    pub(super) application_id: String,
}

enum View {
    Text(Vec<u16>),
    StartupLab(StartupLab),
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
    fn GetMessageW(message: *mut Msg, window: Hwnd, minimum: Uint, maximum: Uint) -> Bool;
    fn TranslateMessage(message: *const Msg) -> Bool;
    fn DispatchMessageW(message: *const Msg) -> Lresult;
    fn PostQuitMessage(exit_code: i32);
    fn BeginPaint(window: Hwnd, paint: *mut PaintStruct) -> Hdc;
    fn EndPaint(window: Hwnd, paint: *const PaintStruct) -> Bool;
    fn GetClientRect(window: Hwnd, rectangle: *mut Rect) -> Bool;
    fn LoadCursorW(instance: Hinstance, cursor_name: *const u16) -> Hcursor;
}

static VIEW: OnceLock<View> = OnceLock::new();
static ACTIVATION_MESSAGE: OnceLock<Uint> = OnceLock::new();

/// Opens the simple host-owned text surface.
pub fn run(title: &str, text: &str) -> io::Result<()> {
    VIEW.set(View::Text(to_wide_null(text)))
        .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "window view was already set"))?;
    run_window(title, 880, 520, None)
}

/// Opens the validated application text surface as the primary host instance.
pub fn run_application(title: &str, text: &str, instance: &PrimaryInstance) -> io::Result<()> {
    VIEW.set(View::Text(to_wide_null(text)))
        .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "window view was already set"))?;
    run_window(title, 880, 520, Some(instance))
}

/// Opens the branded Startup Lab after the caller has completed its checks.
pub fn run_startup_lab(
    display_name: &str,
    application_id: &str,
    instance: &PrimaryInstance,
) -> io::Result<()> {
    VIEW.set(View::StartupLab(StartupLab {
        display_name: display_name.to_owned(),
        application_id: application_id.to_owned(),
    }))
    .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "window view was already set"))?;
    run_window("Anodrel Startup Lab", 1_160, 760, Some(instance))
}

fn run_window(
    title: &str,
    width: i32,
    height: i32,
    primary_instance: Option<&PrimaryInstance>,
) -> io::Result<()> {
    if let Some(primary_instance) = primary_instance {
        ACTIVATION_MESSAGE
            .set(primary_instance.activation_message())
            .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "activation message set"))?;
    }
    let instance = module_handle()?;
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    register_window_class(instance, &class_name)?;
    let title = to_wide_null(title);
    let window = create_window(instance, &class_name, &title, width, height)?;
    if let Some(primary_instance) = primary_instance {
        primary_instance.mark_ready()?;
    }
    show_and_update(window);
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

fn register_window_class(instance: Hinstance, class_name: &[u16]) -> io::Result<()> {
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
                match VIEW.get().expect("window view is initialized") {
                    View::Text(text) => paint::draw_text_surface(device_context, rect, text),
                    View::StartupLab(lab) => startup_lab::draw(device_context, rect, lab),
                }
                // SAFETY: BeginPaint initialized paint for this exact window.
                unsafe {
                    EndPaint(window, &paint);
                }
            }
            0
        }
        WM_DESTROY => {
            // SAFETY: this only posts a quit message to the current thread's
            // queue after the owned top-level window is being destroyed.
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => {
            // SAFETY: all unhandled messages are forwarded unchanged to the
            // documented default Win32 procedure.
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        }
    }
}
