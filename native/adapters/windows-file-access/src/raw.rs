use std::{io, os::windows::ffi::OsStrExt, path::Path, ptr};

use crate::FileIdentity;

type Handle = isize;
type Bool = i32;
type Dword = u32;

const INVALID_HANDLE_VALUE: Handle = -1;
const GENERIC_READ: Dword = 0x8000_0000;
const FILE_SHARE_READ: Dword = 0x0000_0001;
const OPEN_EXISTING: Dword = 3;
const FILE_ATTRIBUTE_NORMAL: Dword = 0x0000_0080;
const FILE_FLAG_OPEN_REPARSE_POINT: Dword = 0x0020_0000;
const FILE_ATTRIBUTE_DIRECTORY: Dword = 0x0000_0010;
const FILE_ATTRIBUTE_REPARSE_POINT: Dword = 0x0000_0400;

#[repr(C)]
struct ByHandleFileInformation {
    attributes: Dword,
    creation_time_low: Dword,
    creation_time_high: Dword,
    access_time_low: Dword,
    access_time_high: Dword,
    write_time_low: Dword,
    write_time_high: Dword,
    volume_serial: Dword,
    file_size_high: Dword,
    file_size_low: Dword,
    number_of_links: Dword,
    file_index_high: Dword,
    file_index_low: Dword,
}

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
    fn GetFileInformationByHandle(file: Handle, information: *mut ByHandleFileInformation) -> Bool;
    fn CloseHandle(handle: Handle) -> Bool;
}

pub(super) struct ReadOnlyFile {
    handle: Handle,
    identity: FileIdentity,
}

impl ReadOnlyFile {
    pub(super) const fn identity(&self) -> FileIdentity {
        self.identity
    }
}

pub(super) fn open_selected_file(path: &Path) -> io::Result<ReadOnlyFile> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "selected path is not absolute",
        ));
    }
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let handle = unsafe {
        // SAFETY: wide is NUL-terminated UTF-16. The synchronous operation
        // permits the null security pointer and no template handle.
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ,
            ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let mut information = std::mem::MaybeUninit::<ByHandleFileInformation>::uninit();
    let captured = unsafe {
        // SAFETY: handle is valid from the successful CreateFileW call and
        // information is writable storage for the exact Windows structure.
        GetFileInformationByHandle(handle, information.as_mut_ptr()) != 0
    };
    if !captured {
        close(handle);
        return Err(io::Error::last_os_error());
    }
    let information = unsafe {
        // SAFETY: GetFileInformationByHandle reported successful initialization.
        information.assume_init()
    };
    if information.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        close(handle);
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "selected object is not a regular file",
        ));
    }
    let file_index =
        (u64::from(information.file_index_high) << 32) | u64::from(information.file_index_low);
    Ok(ReadOnlyFile {
        handle,
        identity: FileIdentity::new(information.volume_serial, file_index),
    })
}

impl Drop for ReadOnlyFile {
    fn drop(&mut self) {
        close(self.handle);
    }
}

fn close(handle: Handle) {
    unsafe {
        // SAFETY: every call owns exactly one successful CreateFileW handle.
        let _ = CloseHandle(handle);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::open_selected_file;

    #[test]
    fn rejects_relative_paths_before_calling_windows() {
        assert!(open_selected_file(Path::new("notes.txt")).is_err());
    }

    #[test]
    fn captures_a_regular_files_stable_identity() {
        let path = std::env::temp_dir().join(format!(
            "anodrel-selected-file-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos(),
        ));
        std::fs::write(&path, "selected").expect("fixture is written");
        let file = open_selected_file(&path).expect("fixture can be captured");
        assert_ne!(file.identity().file_index(), 0);
        drop(file);
        std::fs::remove_file(&path).expect("fixture is removed");
    }
}
