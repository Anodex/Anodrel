//! Narrow User32 confirmation dialog for one signed update candidate.

use std::{ffi::OsStr, os::windows::ffi::OsStrExt};

use anodrel_windows_installer::PackageVersion;

const ID_NO: i32 = 7;
const ID_YES: i32 = 6;
const MB_DEFBUTTON2: u32 = 0x0000_0100;
const MB_ICONINFORMATION: u32 = 0x0000_0040;
const MB_YESNO: u32 = 0x0000_0004;
const UPDATE_CAPTION: &str = "Anodrel update";

/// One direct fixed update confirmation outcome from Windows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeConsent {
    Approved,
    Declined,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(
        owner: *mut core::ffi::c_void,
        text: *const u16,
        caption: *const u16,
        style: u32,
    ) -> i32;
}

/// Shows the only supported host-owned update confirmation on its UI thread.
pub(super) fn request(version: PackageVersion) -> Result<NativeConsent, ()> {
    let text = wide(&dialog_text(version));
    let caption = wide(UPDATE_CAPTION);
    // SAFETY: both UTF-16 buffers are owned and null-terminated for the entire
    // synchronous call. The owner is intentionally null in this first native
    // slice; callers must invoke it from their host UI thread.
    let result = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            dialog_style(),
        )
    };
    match result {
        ID_YES => Ok(NativeConsent::Approved),
        ID_NO => Ok(NativeConsent::Declined),
        _ => Err(()),
    }
}

fn dialog_text(version: PackageVersion) -> String {
    format!(
        "An update to version {}.{}.{} is ready.\r\n\r\nDownload and install it now?",
        version.major(),
        version.minor(),
        version.patch()
    )
}

const fn dialog_style() -> u32 {
    MB_YESNO | MB_ICONINFORMATION | MB_DEFBUTTON2
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use anodrel_windows_installer::PackageVersion;

    use super::{dialog_style, dialog_text};

    #[test]
    fn dialog_shows_only_the_signed_candidate_version_and_defaults_to_no() {
        assert_eq!(
            dialog_text(PackageVersion::new(2, 14, 7)),
            "An update to version 2.14.7 is ready.\r\n\r\nDownload and install it now?"
        );
        assert_eq!(dialog_style(), 0x0000_0144);
    }
}
