//! Fixed native Apps & features removal confirmation and UAC handoff.

use std::{
    ffi::{OsStr, c_void},
    os::windows::ffi::OsStrExt,
};

use anodrel_windows_installer::{UninstallPreflightError, verify_current_uninstall_target};
use anodrel_windows_policy::PolicyStoreError;

const ID_NO: i32 = 7;
const ID_YES: i32 = 6;
const MB_DEFBUTTON2: u32 = 0x0000_0100;
const MB_ICONWARNING: u32 = 0x0000_0030;
const MB_YESNO: u32 = 0x0000_0004;
const SEE_MASK_NOCLOSEPROCESS: u32 = 0x0000_0040;
const SEE_MASK_NOASYNC: u32 = 0x0000_0100;
const SW_SHOWNORMAL: i32 = 1;
const WAIT_OBJECT_0: u32 = 0;
const INFINITE: u32 = u32::MAX;
const ERROR_CANCELLED: u32 = 1223;
const REMOVE_CAPTION: &str = "Anodrel removal";
const REMOVE_TEXT: &str = "Remove the selected Anodrel application for all users?";
type Handle = isize;
const RUN_AS: &[u16] = &[
    b'r' as u16,
    b'u' as u16,
    b'n' as u16,
    b'a' as u16,
    b's' as u16,
    0,
];
const UNINSTALL_ARGUMENT: &[u16] = &[
    b'u' as u16,
    b'n' as u16,
    b'i' as u16,
    b'n' as u16,
    b's' as u16,
    b't' as u16,
    b'a' as u16,
    b'l' as u16,
    b'l' as u16,
    0,
];

#[repr(C)]
struct ShellExecuteInfoW {
    size: u32,
    mask: u32,
    window: *mut c_void,
    verb: *const u16,
    file: *const u16,
    parameters: *const u16,
    directory: *const u16,
    show: i32,
    application: *mut c_void,
    id_list: *mut c_void,
    class: *const u16,
    class_key: *mut c_void,
    hot_key: u32,
    icon: *mut c_void,
    process: Handle,
}

#[link(name = "User32")]
unsafe extern "system" {
    fn MessageBoxW(owner: *mut c_void, text: *const u16, caption: *const u16, style: u32) -> i32;
}

#[link(name = "Kernel32")]
unsafe extern "system" {
    fn CloseHandle(handle: Handle) -> i32;
    fn GetExitCodeProcess(process: Handle, exit_code: *mut u32) -> i32;
    fn GetLastError() -> u32;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
}

#[link(name = "Shell32")]
unsafe extern "system" {
    fn ShellExecuteExW(info: *mut ShellExecuteInfoW) -> i32;
}

/// Runs the sole normal-user product removal route.
pub(super) fn run() -> Result<String, String> {
    verify_current_uninstall_target().map_err(display_error)?;
    if !request_confirmation()? {
        return Ok("Anodrel removal was not started.".to_owned());
    }
    let process = launch_elevated_uninstall()?;
    if !wait_for_success(process)? {
        return Err("the elevated removal did not complete".to_owned());
    }
    verify_policy_absent()?;
    Ok("Anodrel removal completed; final cleanup runs at the next restart.".to_owned())
}

fn request_confirmation() -> Result<bool, String> {
    let text = wide(REMOVE_TEXT);
    let caption = wide(REMOVE_CAPTION);
    let result = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        )
    };
    match result {
        ID_YES => Ok(true),
        ID_NO => Ok(false),
        _ => Err("the removal confirmation could not be displayed".to_owned()),
    }
}

fn launch_elevated_uninstall() -> Result<Handle, String> {
    let image = std::env::current_exe()
        .map_err(|_| "the selected uninstaller image is unavailable".to_owned())?;
    if !image.is_absolute() {
        return Err("the selected uninstaller image is unavailable".to_owned());
    }
    let mut image = image.as_os_str().encode_wide().collect::<Vec<_>>();
    image.push(0);
    let mut info = ShellExecuteInfoW {
        size: std::mem::size_of::<ShellExecuteInfoW>() as u32,
        mask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        window: std::ptr::null_mut(),
        verb: RUN_AS.as_ptr(),
        file: image.as_ptr(),
        parameters: UNINSTALL_ARGUMENT.as_ptr(),
        directory: std::ptr::null(),
        show: SW_SHOWNORMAL,
        application: std::ptr::null_mut(),
        id_list: std::ptr::null_mut(),
        class: std::ptr::null(),
        class_key: std::ptr::null_mut(),
        hot_key: 0,
        icon: std::ptr::null_mut(),
        process: 0,
    };
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(match unsafe { GetLastError() } {
            ERROR_CANCELLED => "Anodrel removal was not approved.".to_owned(),
            _ => "the elevated removal could not be started".to_owned(),
        });
    }
    (info.process != 0)
        .then_some(info.process)
        .ok_or("the elevated removal process could not be observed".to_owned())
}

fn wait_for_success(process: Handle) -> Result<bool, String> {
    let _guard = ProcessHandle(process);
    if unsafe { WaitForSingleObject(process, INFINITE) } != WAIT_OBJECT_0 {
        return Err("the elevated removal process could not be observed".to_owned());
    }
    let mut exit_code = 1_u32;
    if unsafe { GetExitCodeProcess(process, &mut exit_code) } == 0 {
        return Err("the elevated removal process could not be observed".to_owned());
    }
    Ok(exit_code == 0)
}

fn verify_policy_absent() -> Result<(), String> {
    match verify_current_uninstall_target() {
        Err(UninstallPreflightError::InstalledPolicyInvalid(PolicyStoreError::RecordNotFound)) => {
            Ok(())
        }
        _ => Err("the elevated removal did not clear selected policy".to_owned()),
    }
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

struct ProcessHandle(Handle);

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MB_DEFBUTTON2, MB_ICONWARNING, MB_YESNO, REMOVE_TEXT, RUN_AS, UNINSTALL_ARGUMENT};

    #[test]
    fn removal_dialog_defaults_to_cancel_and_handoff_uses_only_uninstall() {
        assert_eq!(
            REMOVE_TEXT,
            "Remove the selected Anodrel application for all users?"
        );
        assert_eq!(MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2, 0x0000_0134);
        assert_eq!(RUN_AS, &[114, 117, 110, 97, 115, 0]);
        assert_eq!(
            UNINSTALL_ARGUMENT,
            &[117, 110, 105, 110, 115, 116, 97, 108, 108, 0]
        );
    }
}
