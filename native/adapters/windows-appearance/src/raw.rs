#![allow(non_snake_case)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::{io, mem, ptr};

use crate::{Rgb, SystemColors};

type Bool = i32;
type Uint = u32;

const SPI_GETHIGHCONTRAST: Uint = 0x0042;
const HCF_HIGHCONTRASTON: Uint = 0x0000_0001;
const COLOR_WINDOW: i32 = 5;
const COLOR_WINDOWTEXT: i32 = 8;
const COLOR_HIGHLIGHT: i32 = 13;
const COLOR_HIGHLIGHTTEXT: i32 = 14;
const COLOR_BTNFACE: i32 = 15;
const COLOR_BTNTEXT: i32 = 18;

#[repr(C)]
struct HighContrast {
    cbSize: Uint,
    dwFlags: Uint,
    lpszDefaultScheme: *mut u16,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn SystemParametersInfoW(
        action: Uint,
        parameter: Uint,
        value: *mut core::ffi::c_void,
        flags: Uint,
    ) -> Bool;
    fn GetSysColor(index: i32) -> Uint;
}

pub fn high_contrast_enabled() -> io::Result<bool> {
    let mut value = HighContrast {
        cbSize: mem::size_of::<HighContrast>() as Uint,
        dwFlags: 0,
        lpszDefaultScheme: ptr::null_mut(),
    };
    // SAFETY: value points to a correctly sized writable HIGHCONTRASTW
    // structure for the duration of the direct User32 query.
    if unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            value.cbSize,
            (&mut value as *mut HighContrast).cast(),
            0,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(value.dwFlags & HCF_HIGHCONTRASTON != 0)
}

pub fn system_colors() -> SystemColors {
    SystemColors {
        window: color(COLOR_WINDOW),
        window_text: color(COLOR_WINDOWTEXT),
        button_face: color(COLOR_BTNFACE),
        button_text: color(COLOR_BTNTEXT),
        highlight: color(COLOR_HIGHLIGHT),
        highlight_text: color(COLOR_HIGHLIGHTTEXT),
    }
}

fn color(index: i32) -> Rgb {
    // SAFETY: GetSysColor accepts only a fixed documented colour index and
    // returns a value rather than writing through a caller pointer.
    colorref_to_rgb(unsafe { GetSysColor(index) })
}

const fn colorref_to_rgb(value: Uint) -> Rgb {
    Rgb {
        red: (value & 0x0000_00FF) as u8,
        green: ((value >> 8) & 0x0000_00FF) as u8,
        blue: ((value >> 16) & 0x0000_00FF) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_windows_bgr_colorrefs_to_rgb() {
        assert_eq!(
            colorref_to_rgb(0x001E_140A),
            Rgb {
                red: 10,
                green: 20,
                blue: 30,
            }
        );
    }
}
