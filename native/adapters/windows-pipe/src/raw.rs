//! Direct, minimal Win32 and CNG bindings for the named-pipe adapter.

#![allow(non_snake_case)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::{io, ptr};

pub type HandleValue = isize;
type Bool = i32;
type Dword = u32;

pub const PIPE_BUFFER_BYTES: usize = 4 * 1024;
const INVALID_HANDLE_VALUE: HandleValue = -1;
const PIPE_ACCESS_DUPLEX: Dword = 0x0000_0003;
const FILE_FLAG_FIRST_PIPE_INSTANCE: Dword = 0x0008_0000;
const PIPE_TYPE_BYTE: Dword = 0x0000_0000;
const PIPE_READMODE_BYTE: Dword = 0x0000_0000;
const PIPE_WAIT: Dword = 0x0000_0000;
#[cfg(test)]
const PIPE_CLIENT_ACCESS: Dword = 0x0012_019B;
#[cfg(test)]
const OPEN_EXISTING: Dword = 3;
const ERROR_PIPE_CONNECTED: i32 = 535;
#[cfg(test)]
const ERROR_PIPE_BUSY: i32 = 231;
const ERROR_BROKEN_PIPE: i32 = 109;
const BCRYPT_USE_SYSTEM_PREFERRED_RNG: Dword = 0x0000_0002;

#[repr(C)]
pub struct SecurityAttributes {
    pub nLength: Dword,
    pub lpSecurityDescriptor: *mut core::ffi::c_void,
    pub bInheritHandle: Bool,
}

#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(
        algorithm: HandleValue,
        buffer: *mut u8,
        buffer_length: Dword,
        flags: Dword,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateNamedPipeW(
        name: *const u16,
        open_mode: Dword,
        pipe_mode: Dword,
        max_instances: Dword,
        output_buffer_size: Dword,
        input_buffer_size: Dword,
        default_timeout: Dword,
        security_attributes: *const SecurityAttributes,
    ) -> HandleValue;
    fn ConnectNamedPipe(pipe: HandleValue, overlapped: *mut core::ffi::c_void) -> Bool;
    fn DisconnectNamedPipe(pipe: HandleValue) -> Bool;
    #[cfg(test)]
    fn CreateFileW(
        name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *const core::ffi::c_void,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: HandleValue,
    ) -> HandleValue;
    #[cfg(test)]
    fn WaitNamedPipeW(name: *const u16, timeout: Dword) -> Bool;
    fn ReadFile(
        file: HandleValue,
        buffer: *mut core::ffi::c_void,
        bytes_to_read: Dword,
        bytes_read: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn WriteFile(
        file: HandleValue,
        buffer: *const core::ffi::c_void,
        bytes_to_write: Dword,
        bytes_written: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn CloseHandle(handle: HandleValue) -> Bool;
    fn LocalFree(memory: HandleValue) -> HandleValue;
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

    pub const fn value(&self) -> HandleValue {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: OwnedHandle is constructed only from a successful Win32
        // creator and closes its unique handle exactly once during drop.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn random_bytes(output: &mut [u8]) -> io::Result<()> {
    let length = u32::try_from(output.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "random buffer is too large"))?;
    // SAFETY: BCRYPT_USE_SYSTEM_PREFERRED_RNG permits a null algorithm handle;
    // output is writable storage for exactly the declared byte count.
    let status = unsafe {
        BCryptGenRandom(
            0,
            output.as_mut_ptr(),
            length,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "BCryptGenRandom failed with NTSTATUS {status:#x}"
        )))
    }
}

pub fn create_server_pipe(
    name: &[u16],
    attributes: &SecurityAttributes,
) -> io::Result<OwnedHandle> {
    // SAFETY: name is a null-terminated UTF-16 pipe name and attributes points
    // to an initialized descriptor that remains live for this call.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            1,
            PIPE_BUFFER_BYTES as Dword,
            PIPE_BUFFER_BYTES as Dword,
            0,
            attributes,
        )
    };
    OwnedHandle::new(handle)
}

pub fn connect_server(handle: &OwnedHandle) -> io::Result<()> {
    // SAFETY: handle belongs to a synchronous named-pipe server and null
    // requests the matching synchronous operation.
    if unsafe { ConnectNamedPipe(handle.value(), ptr::null_mut()) } != 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_PIPE_CONNECTED) {
        Ok(())
    } else {
        Err(error)
    }
}

pub fn disconnect_server(handle: &OwnedHandle) {
    // SAFETY: the pipe handle remains owned for the duration of the call; a
    // disconnect failure during cleanup cannot recover a closed peer.
    unsafe {
        DisconnectNamedPipe(handle.value());
    }
}

#[cfg(test)]
pub fn connect_client(name: &[u16]) -> io::Result<OwnedHandle> {
    for _ in 0..2 {
        // SAFETY: name is a null-terminated UTF-16 pipe name. The client asks
        // only for data read/write and synchronization, not generic write.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                PIPE_CLIENT_ACCESS,
                0,
                ptr::null(),
                OPEN_EXISTING,
                0,
                0,
            )
        };
        if let Ok(handle) = OwnedHandle::new(handle) {
            return Ok(handle);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_PIPE_BUSY) {
            return Err(error);
        }
        // SAFETY: name remains valid through this bounded wait for an existing
        // pipe instance to become available.
        if unsafe { WaitNamedPipeW(name.as_ptr(), 1_000) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        "named pipe remained busy after the bounded wait",
    ))
}

pub fn read(handle: &OwnedHandle, output: &mut [u8]) -> io::Result<usize> {
    let byte_count = u32::try_from(output.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "read buffer is too large"))?;
    let mut bytes_read = 0;
    // SAFETY: output is writable for exactly byte_count bytes and the named-pipe
    // handle remains valid for the synchronous read.
    if unsafe {
        ReadFile(
            handle.value(),
            output.as_mut_ptr().cast(),
            byte_count,
            &mut bytes_read,
            ptr::null_mut(),
        )
    } != 0
    {
        Ok(bytes_read as usize)
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn write_all(handle: &OwnedHandle, input: &[u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset != input.len() {
        let remaining = &input[offset..];
        let byte_count = u32::try_from(remaining.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pipe write is too large"))?;
        let mut bytes_written = 0;
        // SAFETY: remaining stays valid through this synchronous call and the
        // pipe handle is owned for the call's duration.
        let succeeded = unsafe {
            WriteFile(
                handle.value(),
                remaining.as_ptr().cast(),
                byte_count,
                &mut bytes_written,
                ptr::null_mut(),
            )
        } != 0;
        if !succeeded {
            return Err(io::Error::last_os_error());
        }
        if bytes_written == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "named pipe wrote zero bytes",
            ));
        }
        offset += bytes_written as usize;
    }
    Ok(())
}

pub fn is_broken_pipe(error: &io::Error) -> bool {
    error.raw_os_error() == Some(ERROR_BROKEN_PIPE)
}

pub fn free_local(memory: *mut core::ffi::c_void) {
    // SAFETY: ConvertStringSecurityDescriptorToSecurityDescriptorW allocates
    // this memory with LocalAlloc, which must be released with LocalFree.
    unsafe {
        LocalFree(memory as HandleValue);
    }
}
