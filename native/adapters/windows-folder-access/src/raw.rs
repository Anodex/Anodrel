use std::{io, os::windows::ffi::OsStrExt, path::Path, ptr};

use anodrel_folder_access::FolderReference;

use crate::FolderIdentity;

pub(super) type Handle = isize;
pub(super) type Bool = i32;
pub(super) type Dword = u32;

const INVALID_HANDLE_VALUE: Handle = -1;
const FILE_LIST_DIRECTORY: Dword = 0x0000_0001;
const FILE_READ_ATTRIBUTES: Dword = 0x0000_0080;
const FILE_SHARE_READ: Dword = 0x0000_0001;
const OPEN_EXISTING: Dword = 3;
const FILE_FLAG_BACKUP_SEMANTICS: Dword = 0x0200_0000;
const FILE_FLAG_OPEN_REPARSE_POINT: Dword = 0x0020_0000;
pub(super) const FILE_ATTRIBUTE_DIRECTORY: Dword = 0x0000_0010;
pub(super) const FILE_ATTRIBUTE_REPARSE_POINT: Dword = 0x0000_0400;
const BCRYPT_USE_SYSTEM_PREFERRED_RNG: Dword = 0x0000_0002;

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
    pub(super) fn GetFileInformationByHandleEx(
        file: Handle,
        information_class: Dword,
        information: *mut core::ffi::c_void,
        buffer_size: Dword,
    ) -> Bool;
    pub(super) fn CloseHandle(handle: Handle) -> Bool;
}

#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(
        algorithm: Handle,
        buffer: *mut u8,
        buffer_length: Dword,
        flags: Dword,
    ) -> i32;
}

/// One adapter-private selected directory handle and its captured identity.
pub(super) struct RetainedDirectory {
    handle: Handle,
    identity: FolderIdentity,
}

impl RetainedDirectory {
    pub(super) const fn handle(&self) -> Handle {
        self.handle
    }

    pub(super) const fn identity(&self) -> FolderIdentity {
        self.identity
    }
}

/// Captures a single selected non-reparse directory and retains its handle.
pub(super) fn open_selected_folder(path: &Path) -> io::Result<RetainedDirectory> {
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
        // SAFETY: wide is NUL-terminated UTF-16. This opens an existing
        // directory only, with no security attributes or template handle.
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let information = match file_information(handle) {
        Ok(information) => information,
        Err(error) => {
            close(handle);
            return Err(error);
        }
    };
    if information.attributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || information.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        close(handle);
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "selected object is not a regular directory",
        ));
    }
    let file_index =
        (u64::from(information.file_index_high) << 32) | u64::from(information.file_index_low);
    Ok(RetainedDirectory {
        handle,
        identity: FolderIdentity::new(information.volume_serial, file_index),
    })
}

/// Creates one CNG-backed opaque reference after successful folder capture.
pub(super) fn new_folder_reference() -> io::Result<FolderReference> {
    let mut bytes = [0_u8; 16];
    let status = unsafe {
        // SAFETY: the system-preferred RNG permits a null algorithm handle and
        // bytes is writable storage for the exact declared byte count.
        BCryptGenRandom(
            0,
            bytes.as_mut_ptr(),
            bytes.len() as Dword,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status != 0 {
        return Err(io::Error::other("Windows random generation failed"));
    }
    FolderReference::new(base64url_128(&bytes))
        .map_err(|_| io::Error::other("Windows random reference was malformed"))
}

fn file_information(handle: Handle) -> io::Result<ByHandleFileInformation> {
    let mut information = std::mem::MaybeUninit::<ByHandleFileInformation>::uninit();
    let captured = unsafe {
        // SAFETY: handle is valid from the successful CreateFileW call and
        // information is writable storage for the exact Windows structure.
        GetFileInformationByHandle(handle, information.as_mut_ptr()) != 0
    };
    if !captured {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe {
        // SAFETY: GetFileInformationByHandle reported successful initialization.
        information.assume_init()
    })
}

fn base64url_128(bytes: &[u8; 16]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::with_capacity(22);
    for chunk in bytes[..15].chunks_exact(3) {
        encoded.push(ALPHABET[(chunk[0] >> 2) as usize] as char);
        encoded.push(ALPHABET[((chunk[0] & 0b0000_0011) << 4 | chunk[1] >> 4) as usize] as char);
        encoded.push(ALPHABET[((chunk[0] & 0b0000_1111) << 2 | chunk[2] >> 6) as usize] as char);
        encoded.push(ALPHABET[(chunk[2] & 0b0011_1111) as usize] as char);
    }
    let last = bytes[15];
    encoded.push(ALPHABET[(last >> 2) as usize] as char);
    encoded.push(ALPHABET[((last & 0b0000_0011) << 4) as usize] as char);
    encoded
}

impl Drop for RetainedDirectory {
    fn drop(&mut self) {
        close(self.handle);
    }
}

pub(super) fn close(handle: Handle) {
    unsafe {
        // SAFETY: every call owns exactly one successful CreateFileW handle.
        let _ = CloseHandle(handle);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{base64url_128, new_folder_reference, open_selected_folder};

    #[test]
    fn rejects_relative_paths_before_calling_windows() {
        assert!(open_selected_folder(Path::new("fixtures")).is_err());
    }

    #[test]
    fn cng_generated_references_are_exact_and_not_reused() {
        let first = new_folder_reference().expect("reference is generated");
        let second = new_folder_reference().expect("reference is generated");
        assert_eq!(first.as_str().len(), 22);
        assert_ne!(first, second);
    }

    #[test]
    fn encodes_128_bits_as_unpadded_base64url() {
        assert_eq!(base64url_128(&[0; 16]), "AAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(base64url_128(&[255; 16]), "_____________________w");
    }
}
