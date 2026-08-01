//! Minimal Win32 bindings for current-session instance coordination.

#![allow(non_snake_case)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::{io, ptr};

type Bool = i32;
type Dword = u32;
type HandleValue = isize;
type Hwnd = isize;
type Uint = u32;
type Wparam = usize;
type Lparam = isize;

const INVALID_HANDLE_VALUE: HandleValue = -1;
const ERROR_ALREADY_EXISTS: i32 = 183;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const SYNCHRONIZE: Dword = 0x0010_0000;
const WAIT_OBJECT_0: Dword = 0;
const WAIT_TIMEOUT: Dword = 258;
const WAIT_FAILED: Dword = 0xFFFF_FFFF;
const HWND_BROADCAST: Hwnd = 0xFFFF;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(
        security_attributes: *const core::ffi::c_void,
        initially_owned: Bool,
        name: *const u16,
    ) -> HandleValue;
    fn CreateEventW(
        security_attributes: *const core::ffi::c_void,
        manual_reset: Bool,
        initial_state: Bool,
        name: *const u16,
    ) -> HandleValue;
    fn OpenEventW(desired_access: Dword, inherit_handle: Bool, name: *const u16) -> HandleValue;
    fn SetEvent(event: HandleValue) -> Bool;
    fn WaitForSingleObject(handle: HandleValue, milliseconds: Dword) -> Dword;
    fn CloseHandle(handle: HandleValue) -> Bool;
    fn GetLastError() -> Dword;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterWindowMessageW(message_name: *const u16) -> Uint;
    fn PostMessageW(window: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam) -> Bool;
}

pub struct OwnedHandle(HandleValue);

impl OwnedHandle {
    fn new(handle: HandleValue) -> io::Result<Self> {
        if handle == 0 || handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }

    const fn value(&self) -> HandleValue {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: OwnedHandle is created only from successful Win32 handle
        // constructors and closes its unique handle exactly once.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn create_mutex(name: &[u16]) -> io::Result<(OwnedHandle, bool)> {
    // SAFETY: name is a null-terminated UTF-16 kernel-object name and null
    // requests the current process default security attributes.
    let handle = unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
    let handle = OwnedHandle::new(handle)?;
    // SAFETY: GetLastError reads the thread-local status recorded by the
    // immediately preceding CreateMutexW call.
    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS as Dword;
    Ok((handle, already_exists))
}

pub fn create_manual_reset_event(name: &[u16]) -> io::Result<OwnedHandle> {
    // SAFETY: name is a null-terminated UTF-16 kernel-object name. The event
    // starts nonsignaled and uses the current process default security.
    let handle = unsafe { CreateEventW(ptr::null(), 1, 0, name.as_ptr()) };
    OwnedHandle::new(handle)
}

pub fn open_ready_event(name: &[u16]) -> io::Result<OwnedHandle> {
    // SAFETY: name is a null-terminated UTF-16 event name. SYNCHRONIZE grants
    // only the wait right needed by a secondary invocation.
    let handle = unsafe { OpenEventW(SYNCHRONIZE, 0, name.as_ptr()) };
    OwnedHandle::new(handle)
}

pub fn set_event(event: &OwnedHandle) -> io::Result<()> {
    // SAFETY: event is a live owned event handle and SetEvent retains no
    // pointer or handle beyond this synchronous call.
    if unsafe { SetEvent(event.value()) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub enum WaitStatus {
    Signaled,
    Pending,
}

pub fn poll_event(event: &OwnedHandle) -> io::Result<WaitStatus> {
    // SAFETY: event is a live owned waitable handle and the zero timeout makes
    // this a non-blocking state check.
    match unsafe { WaitForSingleObject(event.value(), 0) } {
        WAIT_OBJECT_0 => Ok(WaitStatus::Signaled),
        WAIT_TIMEOUT => Ok(WaitStatus::Pending),
        WAIT_FAILED => Err(io::Error::last_os_error()),
        _ => Err(io::Error::other("unexpected instance readiness result")),
    }
}

pub fn register_activation_message(name: &[u16]) -> io::Result<Uint> {
    // SAFETY: name is a null-terminated UTF-16 string retained only for this
    // call. Windows assigns the registered message identifier system-wide.
    let message = unsafe { RegisterWindowMessageW(name.as_ptr()) };
    if message == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(message)
    }
}

pub fn post_activation_message(message: Uint) -> io::Result<()> {
    // SAFETY: HWND_BROADCAST sends the host-registered message without
    // pointers or payload to top-level windows in the current desktop.
    if unsafe { PostMessageW(HWND_BROADCAST, message, 0, 0) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn is_not_found(error: &io::Error) -> bool {
    error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND)
}
