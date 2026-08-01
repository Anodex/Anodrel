//! Narrow direct Kernel32 bindings for an executable verification lock.

use std::{
    io::{self, Read},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr,
};

type Handle = isize;
type Bool = i32;
type Dword = u32;

const INVALID_HANDLE_VALUE: Handle = -1;
const GENERIC_READ: Dword = 0x8000_0000;
const FILE_SHARE_READ: Dword = 0x0000_0001;
const OPEN_EXISTING: Dword = 3;
const FILE_ATTRIBUTE_NORMAL: Dword = 0x0000_0080;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *const core::ffi::c_void,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: Handle,
    ) -> Handle;
    fn ReadFile(
        file: Handle,
        buffer: *mut core::ffi::c_void,
        bytes_to_read: Dword,
        bytes_read: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn CloseHandle(handle: Handle) -> Bool;
}

/// A read handle that prevents a later write, delete, or rename handle from
/// opening the executable while verification and process creation are running.
pub(super) struct LockedExecutable {
    handle: Handle,
    path: PathBuf,
}

impl LockedExecutable {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

/// Opens an absolute executable with sharing limited to readers.
pub(super) fn lock_executable(path: &Path) -> io::Result<LockedExecutable> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "registered executable path is not absolute",
        ));
    }
    let name = wide_null(path);
    let handle = unsafe {
        // SAFETY: name is NUL-terminated UTF-16, and every other pointer is
        // null only where the synchronous CreateFileW contract permits it.
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ,
            ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(LockedExecutable {
            handle,
            path: path.to_path_buf(),
        })
    }
}

impl Read for LockedExecutable {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let requested = buffer.len().min(u32::MAX as usize) as u32;
        let mut read = 0_u32;
        let success = unsafe {
            // SAFETY: buffer is writable for requested bytes, the file handle
            // remains live for this synchronous read, and no OVERLAPPED state
            // is used for this synchronous handle.
            ReadFile(
                self.handle,
                buffer.as_mut_ptr().cast(),
                requested,
                &mut read,
                ptr::null_mut(),
            )
        };
        if success == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(read as usize)
        }
    }
}

impl Drop for LockedExecutable {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: LockedExecutable is constructed only from a successful
            // CreateFileW call and closes exactly that executable handle.
            let _ = CloseHandle(self.handle);
        }
    }
}

fn wide_null(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        fs::OpenOptions,
        io::Read,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::lock_executable;

    #[test]
    fn relative_paths_are_rejected_before_kernel32_is_called() {
        assert!(lock_executable(Path::new("bin/sample.exe")).is_err());
    }

    #[test]
    fn a_locked_file_can_be_read_through_its_same_handle() {
        let path = std::env::temp_dir().join(format!(
            "anodrel-launch-lock-{}-{}.bin",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is after epoch")
                .as_nanos()
        ));
        std::fs::write(&path, b"locked executable bytes").expect("fixture is written");
        let mut locked = lock_executable(&path).expect("fixture is locked");
        let mut bytes = Vec::new();
        locked.read_to_end(&mut bytes).expect("fixture is read");
        assert_eq!(bytes, b"locked executable bytes");
        assert!(
            OpenOptions::new().write(true).open(&path).is_err(),
            "a competing writer must be rejected while verification holds the lock"
        );
        drop(locked);
        std::fs::remove_file(&path).expect("fixture is removed");
    }
}
