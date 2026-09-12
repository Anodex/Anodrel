//! Final native notification from the independently verified cleanup helper.

use anodrel_windows_installer::run_current_uninstall_cleanup;

#[link(name = "User32")]
unsafe extern "system" {
    fn MessageBoxW(
        window: *mut core::ffi::c_void,
        text: *const u16,
        caption: *const u16,
        flags: u32,
    ) -> i32;
}
pub(super) fn run() -> Result<String, String> {
    let result = run_current_uninstall_cleanup();
    let message = match result {
        Ok(()) => "Anodrel application removed. No Windows restart is required.".to_owned(),
        Err(error) => error.to_string(),
    };
    let text = message.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let caption = "Anodrel removal"
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: Local terminated strings; a standalone notification has no owner.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            if result.is_ok() { 0x40 } else { 0x10 },
        );
    }
    result
        .map(|()| String::new())
        .map_err(|error| error.to_string())
}
