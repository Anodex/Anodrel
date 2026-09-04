//! Narrow direct Shell32 and User32 bindings for one notification-area entry.

#![allow(non_snake_case)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::io;

pub type Handle = isize;
type Bool = i32;
type Dword = u32;

/// Fixed UTF-16 capacity of `NOTIFYICONDATAW`'s tooltip, including its
/// terminator. The safe wrapper validates host text one unit below this bound.
pub const TIP_UNITS: usize = 128;
pub const BODY_UNITS: usize = 256;
pub const TITLE_UNITS: usize = 64;

const NIM_ADD: Dword = 0x0000_0000;
const NIM_MODIFY: Dword = 0x0000_0001;
const NIM_DELETE: Dword = 0x0000_0002;

const NIF_ICON: Dword = 0x0000_0002;
const NIF_TIP: Dword = 0x0000_0004;
const NIF_INFO: Dword = 0x0000_0010;
const NIF_MESSAGE: Dword = 0x0000_0001;

const NIIF_NONE: Dword = 0x0000_0000;
const NIIF_NOSOUND: Dword = 0x0000_0010;
const IDI_APPLICATION: usize = 32_512;
const ICON_ID: Dword = 1;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct NotifyIconDataW {
    cbSize: Dword,
    hWnd: Handle,
    uID: Dword,
    uFlags: Dword,
    uCallbackMessage: Dword,
    hIcon: Handle,
    szTip: [u16; TIP_UNITS],
    dwState: Dword,
    dwStateMask: Dword,
    szInfo: [u16; BODY_UNITS],
    uVersionOrTimeout: Dword,
    szInfoTitle: [u16; TITLE_UNITS],
    dwInfoFlags: Dword,
    guidItem: Guid,
    hBalloonIcon: Handle,
}

impl NotifyIconDataW {
    fn new(window: Handle) -> Self {
        Self {
            cbSize: size_of::<Self>() as Dword,
            hWnd: window,
            uID: ICON_ID,
            uFlags: 0,
            uCallbackMessage: 0,
            hIcon: 0,
            szTip: [0; TIP_UNITS],
            dwState: 0,
            dwStateMask: 0,
            szInfo: [0; BODY_UNITS],
            uVersionOrTimeout: 0,
            szInfoTitle: [0; TITLE_UNITS],
            dwInfoFlags: 0,
            guidItem: Guid {
                data1: 0,
                data2: 0,
                data3: 0,
                data4: [0; 8],
            },
            hBalloonIcon: 0,
        }
    }
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn Shell_NotifyIconW(message: Dword, data: *const NotifyIconDataW) -> Bool;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn LoadIconW(instance: Handle, name: *const u16) -> Handle;
}

pub fn default_icon() -> Handle {
    // SAFETY: a null instance with an integer resource identifier requests one
    // shared system icon, which the process never has to free.
    unsafe { LoadIconW(0, IDI_APPLICATION as *const u16) }
}

pub fn add_icon(window: Handle, icon: Handle, tip: &str) -> io::Result<()> {
    let mut data = NotifyIconDataW::new(window);
    data.uFlags = NIF_ICON | NIF_TIP;
    data.hIcon = icon;
    write_field(&mut data.szTip, tip);
    send(NIM_ADD, &data)
}

pub fn show_balloon(window: Handle, title: &str, body: &str) -> io::Result<()> {
    let mut data = NotifyIconDataW::new(window);
    data.uFlags = NIF_INFO;
    data.dwInfoFlags = NIIF_NONE | NIIF_NOSOUND;
    write_field(&mut data.szInfoTitle, title);
    write_field(&mut data.szInfo, body);
    send(NIM_MODIFY, &data)
}

/// Adds one host-private callback message to an already-created entry.
///
/// Shell32 receives only the host window and a private message number. The
/// application never supplies either value, and this low-level binding never
/// dispatches the callback itself.
pub fn set_callback_message(window: Handle, message: Dword) -> io::Result<()> {
    let mut data = NotifyIconDataW::new(window);
    data.uFlags = NIF_MESSAGE;
    data.uCallbackMessage = message;
    send(NIM_MODIFY, &data)
}

pub fn remove_icon(window: Handle) -> io::Result<()> {
    send(NIM_DELETE, &NotifyIconDataW::new(window))
}

fn send(message: Dword, data: &NotifyIconDataW) -> io::Result<()> {
    // SAFETY: `data` is fully initialized, its size matches its layout, and it
    // stays live for Shell32's synchronous call.
    if unsafe { Shell_NotifyIconW(message, data) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn write_field(field: &mut [u16], value: &str) {
    let limit = field.len().saturating_sub(1);
    let mut written = 0;
    for unit in value.encode_utf16().take(limit) {
        field[written] = unit;
        written += 1;
    }
    field[written] = 0;
}

#[cfg(test)]
mod tests {
    use super::{BODY_UNITS, NotifyIconDataW, TITLE_UNITS, write_field};

    #[test]
    fn a_value_is_copied_and_terminated() {
        let mut field = [0_u16; 8];
        write_field(&mut field, "abc");
        assert_eq!(&field[..4], &[b'a' as u16, b'b' as u16, b'c' as u16, 0]);
    }

    #[test]
    fn a_value_that_would_overflow_is_cut_before_the_terminator() {
        let mut field = [0xFFFF_u16; 4];
        write_field(&mut field, "abcdefgh");
        assert_eq!(field, [b'a' as u16, b'b' as u16, b'c' as u16, 0]);
    }

    #[test]
    fn a_surrogate_pair_is_copied_as_two_units() {
        let mut field = [0_u16; 8];
        write_field(&mut field, "\u{1F680}");
        assert_eq!(field[0], 0xD83D);
        assert_eq!(field[1], 0xDE80);
        assert_eq!(field[2], 0);
    }

    #[test]
    fn the_declared_structure_size_matches_its_own_layout() {
        assert_eq!(
            NotifyIconDataW::new(0).cbSize as usize,
            size_of::<NotifyIconDataW>()
        );
    }

    #[test]
    fn field_capacities_match_the_notification_contract() {
        assert_eq!(
            TITLE_UNITS,
            anodrel_notifications::MAX_TITLE_UTF16_UNITS + 1
        );
        assert_eq!(BODY_UNITS, anodrel_notifications::MAX_BODY_UTF16_UNITS + 1);
    }
}
