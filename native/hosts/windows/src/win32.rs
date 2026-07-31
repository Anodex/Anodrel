//! The small direct Win32 surface required by the first Anodrel host.
//!
//! Raw FFI is isolated in this module so the protocol and policy layers remain
//! portable and safe Rust. Each extern call is wrapped at the narrowest useful
//! boundary; no raw handle escapes this module.

#![allow(non_snake_case)]

use std::{io, ptr, sync::OnceLock};

type Atom = u16;
type Bool = i32;
type Dword = u32;
type Hbrush = isize;
type Hcursor = isize;
type Hdc = isize;
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
const WM_DESTROY: Uint = 0x0002;
const WM_PAINT: Uint = 0x000F;
const DT_LEFT: Uint = 0x0000;
const DT_TOP: Uint = 0x0000;
const DT_WORDBREAK: Uint = 0x0010;
const COLOR_WINDOW: isize = 5;
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
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
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
    fn GetMessageW(message: *mut Msg, window: Hwnd, minimum: Uint, maximum: Uint) -> Bool;
    fn TranslateMessage(message: *const Msg) -> Bool;
    fn DispatchMessageW(message: *const Msg) -> Lresult;
    fn PostQuitMessage(exit_code: i32);
    fn BeginPaint(window: Hwnd, paint: *mut PaintStruct) -> Hdc;
    fn EndPaint(window: Hwnd, paint: *const PaintStruct) -> Bool;
    fn GetClientRect(window: Hwnd, rectangle: *mut Rect) -> Bool;
    fn DrawTextW(
        device_context: Hdc,
        text: *const u16,
        text_length: i32,
        rectangle: *mut Rect,
        format: Uint,
    ) -> i32;
    fn LoadCursorW(instance: Hinstance, cursor_name: *const u16) -> Hcursor;
}

static DISPLAY_TEXT: OnceLock<Vec<u16>> = OnceLock::new();

pub fn run(title: &str, text: &str) -> io::Result<()> {
    DISPLAY_TEXT
        .set(to_wide_null(text))
        .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "window text was already set"))?;

    let instance = module_handle()?;
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    register_window_class(instance, &class_name)?;
    let title = to_wide_null(title);
    let window = create_window(instance, &class_name, &title)?;
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
        hbrBackground: COLOR_WINDOW + 1,
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

fn create_window(instance: Hinstance, class_name: &[u16], title: &[u16]) -> io::Result<Hwnd> {
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
            880,
            520,
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
        WM_PAINT => {
            let mut paint = PaintStruct::default();
            // SAFETY: Windows calls this procedure for a valid window, and
            // paint is writable stack storage for the matching EndPaint call.
            let device_context = unsafe { BeginPaint(window, &mut paint) };
            if device_context != 0 {
                let mut rect = Rect::default();
                // SAFETY: rect is writable stack storage; the immutable text is
                // initialized before the window is created and remains live.
                unsafe {
                    GetClientRect(window, &mut rect);
                    let text = DISPLAY_TEXT.get().expect("window text is initialized");
                    DrawTextW(
                        device_context,
                        text.as_ptr(),
                        -1,
                        &mut rect,
                        DT_LEFT | DT_TOP | DT_WORDBREAK,
                    );
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
