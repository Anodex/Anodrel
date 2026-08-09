//! The narrow client half of the Windows named-pipe transport.
//!
//! The host owns pipe creation, its logon-SID access control, and its session
//! credential. A launched child needs only to open the exact endpoint it was
//! invited to and move bytes, so this module binds four Kernel32 entry points
//! and nothing else. It has no pipe name construction, enumeration, security,
//! or server capability.

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case)]

use std::{io, ptr};

type HandleValue = isize;
type Bool = i32;
type Dword = u32;

/// The read buffer size, matching the host endpoint's own buffer.
pub const PIPE_BUFFER_BYTES: usize = 4 * 1024;

const INVALID_HANDLE_VALUE: HandleValue = -1;
/// Data read, data write, and synchronize only: never generic write access.
const PIPE_CLIENT_ACCESS: Dword = 0x0012_019B;
const OPEN_EXISTING: Dword = 3;
const ERROR_PIPE_BUSY: i32 = 231;
const BUSY_WAIT_MILLISECONDS: Dword = 1_000;
const BUSY_ATTEMPTS: usize = 2;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *const core::ffi::c_void,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: HandleValue,
    ) -> HandleValue;
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
}

/// One connected client endpoint, closed exactly once when it is dropped.
pub struct PipeClient(HandleValue);

impl PipeClient {
    /// Opens the exact endpoint named by the host's bootstrap invitation.
    ///
    /// The caller must pass the invitation's pipe name unchanged. A busy
    /// endpoint is retried within one bounded wait; nothing else is retried.
    pub fn connect(pipe_name: &str) -> io::Result<Self> {
        let name = wide_null(pipe_name);
        for _ in 0..BUSY_ATTEMPTS {
            let error = match Self::open(&name) {
                Ok(client) => return Ok(client),
                Err(error) => error,
            };
            if error.raw_os_error() != Some(ERROR_PIPE_BUSY) {
                return Err(error);
            }
            // SAFETY: `name` is NUL-terminated and stays live for this bounded
            // wait for the single host instance to become available.
            if unsafe { WaitNamedPipeW(name.as_ptr(), BUSY_WAIT_MILLISECONDS) } == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "the invited pipe remained busy",
        ))
    }

    fn open(name: &[u16]) -> io::Result<Self> {
        // SAFETY: `name` is a NUL-terminated UTF-16 pipe name that stays live
        // for the call. The client requests only data access and shares nothing.
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
        if handle == 0 || handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }

    /// Reads whatever bytes are currently available, or zero at end of stream.
    pub fn read(&self, output: &mut [u8]) -> io::Result<usize> {
        let byte_count = u32::try_from(output.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "read buffer is too large"))?;
        let mut bytes_read = 0;
        // SAFETY: `output` is writable for exactly `byte_count` bytes and the
        // handle is owned for the duration of this synchronous read.
        let succeeded = unsafe {
            ReadFile(
                self.0,
                output.as_mut_ptr().cast(),
                byte_count,
                &mut bytes_read,
                ptr::null_mut(),
            )
        } != 0;
        if succeeded {
            Ok(bytes_read as usize)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Writes one complete frame, looping until every byte has been accepted.
    pub fn write_all(&self, input: &[u8]) -> io::Result<()> {
        let mut offset = 0;
        while offset != input.len() {
            let remaining = &input[offset..];
            let byte_count = u32::try_from(remaining.len()).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidInput, "pipe write is too large")
            })?;
            let mut bytes_written = 0;
            // SAFETY: `remaining` stays valid through this synchronous call and
            // the handle is owned for its duration.
            let succeeded = unsafe {
                WriteFile(
                    self.0,
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
                    "the invited pipe accepted no bytes",
                ));
            }
            offset += bytes_written as usize;
        }
        Ok(())
    }
}

impl std::fmt::Debug for PipeClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The raw handle is host-session material and never becomes output.
        formatter.write_str("PipeClient(..)")
    }
}

impl Drop for PipeClient {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful CreateFileW and this unique
        // owner closes it exactly once.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::{PipeClient, wide_null};

    #[test]
    fn a_pipe_name_is_converted_to_terminated_utf16() {
        let name = wide_null(r"\\.\pipe\anodrel.v1.test");
        assert_eq!(name.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&name[..name.len() - 1]).expect("the name is valid UTF-16"),
            r"\\.\pipe\anodrel.v1.test"
        );
    }

    #[test]
    fn an_absent_endpoint_fails_instead_of_waiting() {
        // A child that was never invited must fail immediately rather than
        // blocking a host lifecycle behind a bounded busy wait.
        let error = PipeClient::connect(r"\\.\pipe\anodrel.v1.fixture-absent-endpoint-test")
            .expect_err("an unprovisioned endpoint cannot be opened");
        assert_ne!(error.kind(), std::io::ErrorKind::WouldBlock);
    }
}
