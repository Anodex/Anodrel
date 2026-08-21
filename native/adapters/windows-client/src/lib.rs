//! Direct Windows stream opening for one host-invited native child.
//!
//! This adapter opens only the exact named-pipe endpoint already carried by a
//! validated bootstrap invitation. It owns one Kernel32 handle and implements
//! ordinary blocking `Read` and `Write` for `anodrel-client`; it cannot create,
//! enumerate, secure, or select an endpoint. See `docs/NATIVE_CLIENT.md`.

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case)]

use std::{
    io::{self, Read, Write},
    ptr,
};

use anodrel_bootstrap::BootstrapInvitation;

type HandleValue = isize;
type Bool = i32;
type Dword = u32;

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

/// One connected host-invited Windows named-pipe stream.
///
/// The value owns its handle and closes it exactly once on drop. Its only
/// constructor accepts a `BootstrapInvitation`, keeping the endpoint name out
/// of the child application's adapter API.
pub struct WindowsClientStream(HandleValue);

impl WindowsClientStream {
    /// Opens only the endpoint carried by the host's invitation.
    ///
    /// A briefly busy endpoint receives one bounded wait and retry. Any other
    /// opening failure returns immediately for the portable client to collapse
    /// into its safe stream-unavailable category.
    pub fn connect(invitation: &BootstrapInvitation) -> io::Result<Self> {
        Self::connect_exact(invitation.pipe_name())
    }

    fn connect_exact(pipe_name: &str) -> io::Result<Self> {
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
            // wait for the one server endpoint named by the host invitation.
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
}

impl Read for WindowsClientStream {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        let byte_count = u32::try_from(output.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "read buffer is too large"))?;
        let mut bytes_read = 0;
        // SAFETY: `output` is writable for exactly `byte_count` bytes and the
        // handle is owned for this synchronous call.
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
}

impl Write for WindowsClientStream {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.is_empty() {
            return Ok(0);
        }
        let byte_count = u32::try_from(input.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "write buffer is too large")
        })?;
        let mut bytes_written = 0;
        // SAFETY: `input` stays valid for this synchronous call and the handle
        // is owned for its duration. `Write::write_all` in `anodrel-client`
        // handles a partial successful write without ever looping here.
        let succeeded = unsafe {
            WriteFile(
                self.0,
                input.as_ptr().cast(),
                byte_count,
                &mut bytes_written,
                ptr::null_mut(),
            )
        } != 0;
        if !succeeded {
            return Err(io::Error::last_os_error());
        }
        if !input.is_empty() && bytes_written == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "the invited pipe accepted no bytes",
            ));
        }
        Ok(bytes_written as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        // Named pipes do not add a Rust-side buffered layer. The frame has
        // already reached Kernel32 once `WriteFile` returns successfully.
        Ok(())
    }
}

impl std::fmt::Debug for WindowsClientStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The raw handle is host-session material and never becomes output.
        formatter.write_str("WindowsClientStream(..)")
    }
}

impl Drop for WindowsClientStream {
    fn drop(&mut self) {
        // SAFETY: the handle came from one successful `CreateFileW` call and
        // this unique owner closes it exactly once.
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
    use anodrel_bootstrap::BootstrapInvitation;

    use super::{WindowsClientStream, wide_null};

    const PIPE_NAME: &str = r"\\.\pipe\anodrel.v1.windows-client-test";
    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn a_pipe_name_is_converted_to_terminated_utf16() {
        let name = wide_null(PIPE_NAME);
        assert_eq!(name.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&name[..name.len() - 1]).expect("the name is valid UTF-16"),
            PIPE_NAME
        );
    }

    #[test]
    fn an_absent_invited_endpoint_fails_instead_of_waiting() {
        let invitation = BootstrapInvitation::new(PIPE_NAME, "windows-client-test", TOKEN)
            .expect("invitation is valid");
        // A child that was never invited must fail immediately rather than
        // blocking a host lifecycle behind a busy-endpoint wait.
        let error = WindowsClientStream::connect(&invitation)
            .expect_err("an unprovisioned endpoint cannot be opened");
        assert_ne!(error.kind(), std::io::ErrorKind::WouldBlock);
    }
}
