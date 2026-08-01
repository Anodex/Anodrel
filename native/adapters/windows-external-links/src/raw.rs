//! Narrow Shell32 binding for one prevalidated HTTPS link handoff.

use std::ptr;

#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        owner_window: isize,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show_command: i32,
    ) -> isize;
}

const SW_SHOWNORMAL: i32 = 1;

pub(super) fn open(link: &str) -> Result<(), ()> {
    let link = wide_null(link);
    let result = unsafe {
        // SAFETY: link is one NUL-terminated UTF-16 HTTPS value already
        // validated by the portable crate. All command-related arguments are
        // null, so ShellExecuteW receives no verb, parameters, or directory.
        ShellExecuteW(
            0,
            ptr::null(),
            link.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result > 32 { Ok(()) } else { Err(()) }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::wide_null;

    #[test]
    fn link_encoding_has_one_terminal_nul() {
        let encoded = wide_null("https://docs.anodrel.dev/guide");
        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(encoded.iter().filter(|unit| **unit == 0).count(), 1);
    }
}
