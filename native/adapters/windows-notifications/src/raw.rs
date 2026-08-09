//! Narrow direct Shell32 and User32 bindings for one notification icon.

#![allow(non_snake_case)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::io;

pub type Handle = isize;
type Bool = i32;
type Dword = u32;

/// Fixed UTF-16 capacities of the `NOTIFYICONDATAW` text buffers, terminator
/// included. `anodrel-notifications` bounds its values one unit below each of
/// these, which is why a validated value never needs truncating here.
pub const TIP_UNITS: usize = 128;
pub const BODY_UNITS: usize = 256;
pub const TITLE_UNITS: usize = 64;

const NIM_ADD: Dword = 0x0000_0000;
const NIM_MODIFY: Dword = 0x0000_0001;
const NIM_DELETE: Dword = 0x0000_0002;

const NIF_ICON: Dword = 0x0000_0002;
const NIF_TIP: Dword = 0x0000_0004;
const NIF_INFO: Dword = 0x0000_0010;

/// No sound and no additional balloon artwork: the icon below is the only image.
const NIIF_NONE: Dword = 0x0000_0000;
const NIIF_NOSOUND: Dword = 0x0000_0010;

const IDI_APPLICATION: usize = 32_512;

/// The one notification-area entry this process ever creates.
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

/// Returns the shared system application icon.
///
/// This is the fallback when host code supplies no brand icon. It is still
/// host-selected artwork: an application cannot reach it or replace it.
pub fn default_icon() -> Handle {
    // SAFETY: a null instance with an integer resource identifier requests one
    // of the shared system icons, which the process never has to free.
    unsafe { LoadIconW(0, IDI_APPLICATION as *const u16) }
}

/// Adds this process's single notification-area entry for one owner window.
pub fn add_icon(window: Handle, icon: Handle, tip: &str) -> io::Result<()> {
    let mut data = NotifyIconDataW::new(window);
    data.uFlags = NIF_ICON | NIF_TIP;
    data.hIcon = icon;
    write_field(&mut data.szTip, tip);
    send(NIM_ADD, &data)
}

/// Shows one balloon on the already-added entry.
pub fn show_balloon(window: Handle, title: &str, body: &str) -> io::Result<()> {
    let mut data = NotifyIconDataW::new(window);
    data.uFlags = NIF_INFO;
    // The notification stays silent and carries no extra artwork, so it cannot
    // be used to demand attention beyond the text it was granted.
    data.dwInfoFlags = NIIF_NONE | NIIF_NOSOUND;
    write_field(&mut data.szInfoTitle, title);
    write_field(&mut data.szInfo, body);
    send(NIM_MODIFY, &data)
}

/// Removes this process's notification-area entry.
pub fn remove_icon(window: Handle) -> io::Result<()> {
    send(NIM_DELETE, &NotifyIconDataW::new(window))
}

fn send(message: Dword, data: &NotifyIconDataW) -> io::Result<()> {
    // SAFETY: `data` is a fully initialized structure whose `cbSize` matches its
    // own layout, and it stays live for this synchronous call.
    if unsafe { Shell_NotifyIconW(message, data) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Copies text into a fixed UTF-16 field, always leaving a terminator.
///
/// Callers pass values that `anodrel-notifications` already bounded one unit
/// below the field, so the truncation branch is a belt-and-braces guard rather
/// than an expected path. It exists because writing past one of these arrays
/// would corrupt the neighbouring fields of the structure.
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
    fn an_empty_value_still_terminates_the_field() {
        let mut field = [0xFFFF_u16; 4];
        write_field(&mut field, "");
        assert_eq!(field[0], 0);
    }

    #[test]
    fn a_value_that_would_overflow_is_cut_before_the_terminator() {
        // Writing past one of these arrays would corrupt the neighbouring
        // fields of the structure, so the guard must hold even though the
        // portable bounds mean it should never be reached.
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
        // Windows dispatches on `cbSize`, so a value that disagreed with the
        // real layout would silently select a different structure version.
        assert_eq!(
            NotifyIconDataW::new(0).cbSize as usize,
            size_of::<NotifyIconDataW>()
        );
    }

    #[test]
    fn field_capacities_match_the_documented_portable_bounds() {
        // `anodrel-notifications` bounds each value one unit below its field so
        // a validated value always fits with its terminator. If either side
        // moved without the other, a legal value would start being truncated.
        assert_eq!(
            TITLE_UNITS,
            anodrel_notifications::MAX_TITLE_UTF16_UNITS + 1
        );
        assert_eq!(BODY_UNITS, anodrel_notifications::MAX_BODY_UTF16_UNITS + 1);
    }
}
