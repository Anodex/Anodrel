//! Narrow Shell32 and Ole32 bindings for the Windows Local AppData known folder.

use std::{path::PathBuf, ptr};

type Hresult = i32;

const MAX_PATH_UNITS: usize = 32_768;
const FOLDERID_LOCAL_APP_DATA: Guid = Guid {
    data1: 0xF1B3_2785,
    data2: 0x6FBA,
    data3: 0x4FCF,
    data4: [0x9D, 0x55, 0x7B, 0x8E, 0x7F, 0x15, 0x70, 0x91],
};

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(
        folder_id: *const Guid,
        flags: u32,
        token: *mut core::ffi::c_void,
        path: *mut *mut u16,
    ) -> Hresult;
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(memory: *const core::ffi::c_void);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum KnownFolderError {
    Unavailable,
    InvalidPath,
}

pub(super) fn local_application_data_root() -> Result<PathBuf, KnownFolderError> {
    let mut raw_path = ptr::null_mut();
    let status = unsafe {
        // SAFETY: the folder ID is a valid constant, null selects the current
        // user token, and raw_path receives the allocator-owned UTF-16 result.
        SHGetKnownFolderPath(&FOLDERID_LOCAL_APP_DATA, 0, ptr::null_mut(), &mut raw_path)
    };
    if status < 0 || raw_path.is_null() {
        return Err(KnownFolderError::Unavailable);
    }

    let path = unsafe {
        // SAFETY: successful SHGetKnownFolderPath returns a NUL-terminated
        // CoTaskMemAlloc string. The bounded copy completes before it is freed.
        copy_windows_path(raw_path)
    };
    unsafe {
        // SAFETY: raw_path came from SHGetKnownFolderPath and is freed exactly
        // once after its content has been copied.
        CoTaskMemFree(raw_path.cast());
    }
    path
}

unsafe fn copy_windows_path(raw_path: *const u16) -> Result<PathBuf, KnownFolderError> {
    let mut units = Vec::new();
    for index in 0..MAX_PATH_UNITS {
        let unit = unsafe {
            // SAFETY: SHGetKnownFolderPath promised a NUL-terminated UTF-16
            // string. The explicit bound prevents an unbounded scan if that
            // operating-system contract is not met.
            *raw_path.add(index)
        };
        if unit == 0 {
            return decode_windows_path(&units);
        }
        units.push(unit);
    }
    Err(KnownFolderError::InvalidPath)
}

fn decode_windows_path(units: &[u16]) -> Result<PathBuf, KnownFolderError> {
    let value = String::from_utf16(units).map_err(|_| KnownFolderError::InvalidPath)?;
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(KnownFolderError::InvalidPath)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{KnownFolderError, decode_windows_path};

    #[test]
    fn decodes_an_absolute_utf16_windows_path() {
        let units: Vec<u16> = r"C:\Users\Owner\AppData\Local".encode_utf16().collect();
        assert_eq!(
            decode_windows_path(&units).expect("absolute path is accepted"),
            Path::new(r"C:\Users\Owner\AppData\Local")
        );
    }

    #[test]
    fn rejects_relative_or_malformed_known_folder_paths() {
        let relative: Vec<u16> = "AppData\\Local".encode_utf16().collect();
        assert_eq!(
            decode_windows_path(&relative),
            Err(KnownFolderError::InvalidPath)
        );
        assert_eq!(
            decode_windows_path(&[0xD800]),
            Err(KnownFolderError::InvalidPath)
        );
    }
}
