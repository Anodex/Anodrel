//! Narrow Shell32 and Kernel32 calls for one fixed UAC update process.

use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};

use crate::UpdateHandoffError;

type Handle = *mut c_void;

const ERROR_CANCELLED: u32 = 1223;
const SEE_MASK_NOCLOSEPROCESS: u32 = 0x0000_0040;
const SEE_MASK_NOASYNC: u32 = 0x0000_0100;
const SW_SHOWNORMAL: i32 = 1;
const WAIT_OBJECT_0: u32 = 0;
const INFINITE: u32 = u32::MAX;
const RUN_AS: &[u16] = &[
    b'r' as u16,
    b'u' as u16,
    b'n' as u16,
    b'a' as u16,
    b's' as u16,
    0,
];
const UPDATE_ARGUMENT: &[u16] = &[
    b'u' as u16,
    b'p' as u16,
    b'd' as u16,
    b'a' as u16,
    b't' as u16,
    b'e' as u16,
    0,
];

#[repr(C)]
struct ShellExecuteInfoW {
    size: u32,
    mask: u32,
    window: Handle,
    verb: *const u16,
    file: *const u16,
    parameters: *const u16,
    directory: *const u16,
    show: i32,
    application: Handle,
    id_list: *mut c_void,
    class: *const u16,
    class_key: Handle,
    hot_key: u32,
    icon: Handle,
    process: Handle,
}

unsafe extern "system" {
    fn CloseHandle(handle: Handle) -> i32;
    fn GetExitCodeProcess(process: Handle, exit_code: *mut u32) -> i32;
    fn GetLastError() -> u32;
    fn ShellExecuteExW(info: *mut ShellExecuteInfoW) -> i32;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
}

/// One Windows process handle from the fixed elevated updater command.
pub(crate) struct UpdateProcessHandle(Handle);

impl UpdateProcessHandle {
    /// Starts the only supported elevated installer command.
    pub(crate) fn launch(image: &Path) -> Result<Self, UpdateHandoffError> {
        let mut file = image.as_os_str().encode_wide().collect::<Vec<_>>();
        file.push(0);
        let mut info = ShellExecuteInfoW {
            size: std::mem::size_of::<ShellExecuteInfoW>() as u32,
            mask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            window: std::ptr::null_mut(),
            verb: RUN_AS.as_ptr(),
            file: file.as_ptr(),
            parameters: UPDATE_ARGUMENT.as_ptr(),
            directory: std::ptr::null(),
            show: SW_SHOWNORMAL,
            application: std::ptr::null_mut(),
            id_list: std::ptr::null_mut(),
            class: std::ptr::null(),
            class_key: std::ptr::null_mut(),
            hot_key: 0,
            icon: std::ptr::null_mut(),
            process: std::ptr::null_mut(),
        };
        // SAFETY: every field is fixed except the owned null-terminated image
        // path. No caller chooses a shell verb, parameter, directory, class, or
        // window. ShellExecuteExW fills only the retained process-handle slot.
        if unsafe { ShellExecuteExW(&mut info) } == 0 {
            // SAFETY: GetLastError reads this thread's ShellExecuteExW failure.
            return Err(match unsafe { GetLastError() } {
                ERROR_CANCELLED => UpdateHandoffError::UserDeclined,
                _ => UpdateHandoffError::LaunchFailed,
            });
        }
        if info.process.is_null() {
            return Err(UpdateHandoffError::ProcessUnavailable);
        }
        Ok(Self(info.process))
    }

    /// Waits for the owned process and reports only its conventional outcome.
    pub(crate) fn wait(&self) -> Result<bool, UpdateHandoffError> {
        // SAFETY: this handle was returned by ShellExecuteExW and remains owned
        // by this value until its Drop closes it.
        if unsafe { WaitForSingleObject(self.0, INFINITE) } != WAIT_OBJECT_0 {
            return Err(UpdateHandoffError::ProcessWaitFailed);
        }
        let mut exit_code = 1;
        // SAFETY: the process is signalled, the output pointer is valid, and the
        // still-owned process handle remains valid for this call.
        if unsafe { GetExitCodeProcess(self.0, &mut exit_code) } == 0 {
            return Err(UpdateHandoffError::ProcessWaitFailed);
        }
        Ok(exit_code == 0)
    }

    /// Returns whether Windows has not confirmed process completion.
    pub(crate) fn completion_is_unconfirmed(&self) -> bool {
        // SAFETY: the process handle remains owned by this value.
        unsafe { WaitForSingleObject(self.0, 0) != WAIT_OBJECT_0 }
    }
}

impl Drop for UpdateProcessHandle {
    fn drop(&mut self) {
        // SAFETY: this handle was returned by ShellExecuteExW and is closed
        // exactly once when its sole owner drops.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RUN_AS, UPDATE_ARGUMENT};

    #[test]
    fn direct_handoff_uses_only_the_fixed_elevated_command() {
        assert_eq!(RUN_AS, &[114, 117, 110, 97, 115, 0]);
        assert_eq!(UPDATE_ARGUMENT, &[117, 112, 100, 97, 116, 101, 0]);
    }
}
