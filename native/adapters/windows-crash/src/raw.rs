//! The direct Win32 calls this adapter needs, and nothing more.
//!
//! Deliberately a smaller surface than the storage adapter's file layer: this
//! one creates a directory, writes a new file, enumerates one directory, and
//! deletes a file. It shares no code with that layer because the two have
//! different sharing and creation rules and coupling them would make one
//! adapter's needs quietly change the other's behaviour.

use std::{
    ffi::OsStr,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Component, Path, PathBuf},
    ptr,
};

type Handle = isize;
type Bool = i32;
type Dword = u32;

const INVALID_HANDLE_VALUE: Handle = -1;
const GENERIC_WRITE: Dword = 0x4000_0000;
const CREATE_NEW: Dword = 1;
const FILE_ATTRIBUTE_NORMAL: Dword = 0x0000_0080;
const FILE_ATTRIBUTE_DIRECTORY: Dword = 0x0000_0010;
const FILE_ATTRIBUTE_REPARSE_POINT: Dword = 0x0000_0400;
const FILE_FLAG_OPEN_REPARSE_POINT: Dword = 0x0020_0000;
const FILE_FLAG_WRITE_THROUGH: Dword = 0x8000_0000;
const INVALID_FILE_ATTRIBUTES: Dword = u32::MAX;
const ERROR_FILE_NOT_FOUND: Dword = 2;
const ERROR_PATH_NOT_FOUND: Dword = 3;
const ERROR_NO_MORE_FILES: Dword = 18;
const ERROR_ALREADY_EXISTS: Dword = 183;
const MAX_PATH_CHARS: usize = 260;

#[repr(C)]
struct FindDataW {
    attributes: Dword,
    creation_time: [Dword; 2],
    access_time: [Dword; 2],
    write_time: [Dword; 2],
    file_size_high: Dword,
    file_size_low: Dword,
    reserved0: Dword,
    reserved1: Dword,
    file_name: [u16; MAX_PATH_CHARS],
    alternate_file_name: [u16; 14],
}

impl Default for FindDataW {
    fn default() -> Self {
        Self {
            attributes: 0,
            creation_time: [0; 2],
            access_time: [0; 2],
            write_time: [0; 2],
            file_size_high: 0,
            file_size_low: 0,
            reserved0: 0,
            reserved1: 0,
            file_name: [0; MAX_PATH_CHARS],
            alternate_file_name: [0; 14],
        }
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateDirectoryW(path: *const u16, security_attributes: *const core::ffi::c_void) -> Bool;
    fn GetFileAttributesW(path: *const u16) -> Dword;
    fn GetLastError() -> Dword;
    fn CreateFileW(
        name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *const core::ffi::c_void,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: Handle,
    ) -> Handle;
    fn WriteFile(
        file: Handle,
        buffer: *const core::ffi::c_void,
        bytes_to_write: Dword,
        bytes_written: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn FlushFileBuffers(file: Handle) -> Bool;
    fn DeleteFileW(path: *const u16) -> Bool;
    fn CloseHandle(handle: Handle) -> Bool;
    fn FindFirstFileW(pattern: *const u16, data: *mut FindDataW) -> Handle;
    fn FindNextFileW(search: Handle, data: *mut FindDataW) -> Bool;
    fn FindClose(search: Handle) -> Bool;
}

/// Creates every component of `path` that does not exist yet.
///
/// Refuses a relative component rather than resolving it, so a caller cannot
/// reach outside the location it was given.
pub(super) fn ensure_directory_tree(path: &Path) -> Result<(), ()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::Normal(part) => {
                current.push(part);
                ensure_directory(&current)?;
            }
            Component::CurDir | Component::ParentDir => return Err(()),
        }
    }
    Ok(())
}

/// Writes `bytes` to a file that must not already exist.
///
/// `CREATE_NEW` rather than a truncating create: the caller has chosen a name
/// no existing record holds, so a name that is already taken means its view of
/// the directory was wrong and the right answer is to fail, not to overwrite
/// somebody's record. Opened with no sharing and written through, because the
/// process doing this is on its way down.
pub(super) fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), ()> {
    let wide_path = wide(path);
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            GENERIC_WRITE,
            0,
            ptr::null(),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(());
    }
    let result = write_handle(handle, bytes).and_then(|()| flush(handle));
    close(handle);
    result
}

/// Returns the names of the regular files directly inside `directory`.
///
/// Subdirectories and reparse points are skipped rather than reported, and a
/// missing directory is an empty listing rather than a failure: the first crash
/// on a machine happens before the location exists.
pub(super) fn regular_file_names(directory: &Path) -> Result<Vec<String>, ()> {
    let pattern = wide(&directory.join("*"));
    let mut data = FindDataW::default();
    let search = unsafe { FindFirstFileW(pattern.as_ptr(), &raw mut data) };
    if search == INVALID_HANDLE_VALUE {
        return match last_error() {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(Vec::new()),
            _ => Err(()),
        };
    }

    let mut names = Vec::new();
    loop {
        if data.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) == 0
            && let Some(name) = file_name(&data.file_name)
        {
            names.push(name);
        }
        if unsafe { FindNextFileW(search, &raw mut data) } == 0 {
            break;
        }
    }
    let ended_cleanly = last_error() == ERROR_NO_MORE_FILES;
    unsafe {
        FindClose(search);
    }
    if ended_cleanly { Ok(names) } else { Err(()) }
}

/// Deletes one file, treating an already-absent file as success.
pub(super) fn delete_file(path: &Path) -> Result<(), ()> {
    let wide_path = wide(path);
    let deleted = unsafe { DeleteFileW(wide_path.as_ptr()) };
    if deleted != 0 || matches!(last_error(), ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) {
        Ok(())
    } else {
        Err(())
    }
}

fn ensure_directory(path: &Path) -> Result<(), ()> {
    let wide_path = wide(path);
    let created = unsafe { CreateDirectoryW(wide_path.as_ptr(), ptr::null()) };
    if created == 0 && last_error() != ERROR_ALREADY_EXISTS {
        return Err(());
    }
    let attributes = unsafe { GetFileAttributesW(wide_path.as_ptr()) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(());
    }
    // A directory and not a reparse point: following a junction placed here by
    // something else would write records outside the location the host chose.
    if attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
        == FILE_ATTRIBUTE_DIRECTORY
    {
        Ok(())
    } else {
        Err(())
    }
}

/// Reads one fixed-size `WIN32_FIND_DATAW` name field up to its terminator.
fn file_name(raw: &[u16; MAX_PATH_CHARS]) -> Option<String> {
    let length = raw.iter().position(|unit| *unit == 0)?;
    std::ffi::OsString::from_wide(&raw[..length])
        .into_string()
        .ok()
}

fn write_handle(handle: Handle, bytes: &[u8]) -> Result<(), ()> {
    let mut written_total = 0usize;
    while written_total < bytes.len() {
        let remaining = &bytes[written_total..];
        let count = Dword::try_from(remaining.len()).unwrap_or(Dword::MAX);
        let mut written = 0;
        let ok = unsafe {
            WriteFile(
                handle,
                remaining.as_ptr().cast(),
                count,
                &raw mut written,
                ptr::null_mut(),
            )
        };
        if ok == 0 || written == 0 {
            return Err(());
        }
        written_total += written as usize;
    }
    Ok(())
}

fn flush(handle: Handle) -> Result<(), ()> {
    if unsafe { FlushFileBuffers(handle) } == 0 {
        Err(())
    } else {
        Ok(())
    }
}

fn close(handle: Handle) {
    unsafe {
        CloseHandle(handle);
    }
}

fn last_error() -> Dword {
    unsafe { GetLastError() }
}

fn wide(path: &Path) -> Vec<u16> {
    let mut units: Vec<u16> = OsStr::new(path).encode_wide().collect();
    units.push(0);
    units
}
