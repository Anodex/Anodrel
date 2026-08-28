use std::{io, os::windows::ffi::OsStrExt, path::Path, ptr};

use anodrel_file_access::SelectionReference;

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
    fn ReadFile(
        file: Handle,
        buffer: *mut core::ffi::c_void,
        bytes_to_read: Dword,
        bytes_read: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn CloseHandle(handle: Handle) -> Bool;
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

pub(super) struct ReadOnlyFile {
    handle: Handle,
    identity: FileIdentity,
}

impl ReadOnlyFile {
    pub(super) const fn identity(&self) -> FileIdentity {
        self.identity
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum ReadFailure {
    TooLarge,
    Unavailable,
}

pub(super) fn read_bounded(file: &mut ReadOnlyFile, limit: usize) -> Result<Vec<u8>, ReadFailure> {
    let mut output = Vec::with_capacity(limit.min(4096));
    let mut buffer = [0_u8; 4096];
    loop {
        let remaining = limit.saturating_add(1).saturating_sub(output.len());
        let requested = remaining.min(buffer.len());
        let mut read = 0_u32;
        let success = unsafe {
            // SAFETY: the retained handle remains live, buffer is writable for
            // requested bytes, and this is a synchronous read with no OVERLAPPED state.
            ReadFile(
                file.handle,
                buffer.as_mut_ptr().cast(),
                requested as Dword,
                &mut read,
                ptr::null_mut(),
            )
        };
        if success == 0 {
            return Err(ReadFailure::Unavailable);
        }
        if read == 0 {
            return Ok(output);
        }
        output.extend_from_slice(&buffer[..read as usize]);
        if output.len() > limit {
            return Err(ReadFailure::TooLarge);
        }
    }
}

pub(super) fn new_selection_reference() -> io::Result<SelectionReference> {
    SelectionReference::new(new_reference_value()?)
        .map_err(|_| io::Error::other("Windows random reference was malformed"))
}

pub(super) fn new_reference_value() -> io::Result<String> {
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
    Ok(base64url_128(&bytes))
}

fn base64url_128(bytes: &[u8; 16]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::with_capacity(22);
    let (chunks, remainder) = bytes[..15].as_chunks::<3>();
    debug_assert!(
        remainder.is_empty(),
        "the fixed prefix is divisible by three"
    );
    for chunk in chunks {
        encoded.push(ALPHABET[(chunk[0] >> 2) as usize] as char);
        encoded.push(ALPHABET[((chunk[0] & 0b0000_0011) << 4 | chunk[1] >> 4) as usize] as char);
        encoded.push(ALPHABET[((chunk[1] & 0b0000_1111) << 2 | chunk[2] >> 6) as usize] as char);
        encoded.push(ALPHABET[(chunk[2] & 0b0011_1111) as usize] as char);
    }
    let last = bytes[15];
    encoded.push(ALPHABET[(last >> 2) as usize] as char);
    encoded.push(ALPHABET[((last & 0b0000_0011) << 4) as usize] as char);
    encoded
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

    use super::{
        ReadFailure, base64url_128, new_selection_reference, open_selected_file, read_bounded,
    };

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

    #[test]
    fn cng_generated_references_are_exact_and_not_reused() {
        let first = new_selection_reference().expect("reference is generated");
        let second = new_selection_reference().expect("reference is generated");
        assert_eq!(first.as_str().len(), 22);
        assert_ne!(first, second);
    }

    #[test]
    fn encodes_128_bits_as_unpadded_base64url() {
        assert_eq!(base64url_128(&[0; 16]), "AAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(base64url_128(&[255; 16]), "_____________________w");
    }

    #[test]
    fn reads_only_a_bounded_retained_file() {
        let path = std::env::temp_dir().join(format!(
            "anodrel-selected-read-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos(),
        ));
        std::fs::write(&path, b"retained text").expect("fixture is written");
        let mut file = open_selected_file(&path).expect("fixture is captured");
        assert_eq!(
            read_bounded(&mut file, 32).expect("fixture is read"),
            b"retained text"
        );
        drop(file);
        std::fs::write(&path, b"0123456789").expect("fixture is replaced");
        let mut file = open_selected_file(&path).expect("fixture is captured");
        assert_eq!(read_bounded(&mut file, 4), Err(ReadFailure::TooLarge));
        drop(file);
        std::fs::remove_file(&path).expect("fixture is removed");
    }
}
