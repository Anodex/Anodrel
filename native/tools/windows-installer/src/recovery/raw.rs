//! Direct reparse-safe removal of one private staging tree.

use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};

use super::RecoveryCleanupError;

type FindHandle = *mut c_void;

const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_NO_MORE_FILES: u32 = 18;

#[repr(C)]
struct FileTime {
    low: u32,
    high: u32,
}

#[repr(C)]
struct FindData {
    attributes: u32,
    creation: FileTime,
    access: FileTime,
    write: FileTime,
    size_high: u32,
    size_low: u32,
    reserved0: u32,
    reserved1: u32,
    file_name: [u16; 260],
    alternate_name: [u16; 14],
}

#[link(name = "Kernel32")]
unsafe extern "system" {
    fn GetFileAttributesW(path: *const u16) -> u32;
    fn FindFirstFileW(pattern: *const u16, data: *mut FindData) -> FindHandle;
    fn FindNextFileW(handle: FindHandle, data: *mut FindData) -> i32;
    fn FindClose(handle: FindHandle) -> i32;
    fn DeleteFileW(path: *const u16) -> i32;
    fn RemoveDirectoryW(path: *const u16) -> i32;
    fn GetLastError() -> u32;
}

/// Removes one checked private staging tree while refusing every reparse point.
pub(crate) fn remove_normal_tree(root: &Path) -> Result<(), RecoveryCleanupError> {
    let attributes = attributes(root)?;
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(RecoveryCleanupError::ReparsePointRefused);
    }
    if attributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
        return Err(RecoveryCleanupError::RemovalFailed);
    }
    remove_children(root)?;
    let path = wide_path(root);
    // SAFETY: The root was checked as a normal directory after all its checked
    // children were removed by this module.
    (unsafe { RemoveDirectoryW(path.as_ptr()) } != 0)
        .then_some(())
        .ok_or(RecoveryCleanupError::RemovalFailed)
}

fn remove_children(root: &Path) -> Result<(), RecoveryCleanupError> {
    let pattern = wide_path(&root.join("*"));
    let mut data = empty_find_data();
    // SAFETY: Pattern is a NUL-terminated child wildcard and data is writable.
    let handle = unsafe { FindFirstFileW(pattern.as_ptr(), &mut data) };
    if handle as isize == -1 {
        // SAFETY: GetLastError reads only the current thread's last-error slot.
        return if unsafe { GetLastError() } == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(RecoveryCleanupError::RemovalFailed)
        };
    }
    let guard = FindGuard(handle);
    loop {
        remove_entry(root, &data)?;
        // SAFETY: The guard owns one live enumeration handle and data is writable.
        if unsafe { FindNextFileW(guard.0, &mut data) } == 0 {
            // SAFETY: GetLastError reads only the current thread's last-error slot.
            return if unsafe { GetLastError() } == ERROR_NO_MORE_FILES {
                Ok(())
            } else {
                Err(RecoveryCleanupError::RemovalFailed)
            };
        }
    }
}

fn remove_entry(root: &Path, data: &FindData) -> Result<(), RecoveryCleanupError> {
    let name = utf16_name(&data.file_name).ok_or(RecoveryCleanupError::RemovalFailed)?;
    if name == "." || name == ".." {
        return Ok(());
    }
    if data.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(RecoveryCleanupError::ReparsePointRefused);
    }
    let path = root.join(name);
    if data.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return remove_normal_tree(&path);
    }
    let path = wide_path(&path);
    // SAFETY: The entry came from the guarded current directory enumeration and
    // was checked not to be a directory or reparse point.
    (unsafe { DeleteFileW(path.as_ptr()) } != 0)
        .then_some(())
        .ok_or(RecoveryCleanupError::RemovalFailed)
}

fn attributes(path: &Path) -> Result<u32, RecoveryCleanupError> {
    let path = wide_path(path);
    // SAFETY: The path is NUL terminated and valid for the duration of the call.
    let value = unsafe { GetFileAttributesW(path.as_ptr()) };
    (value != INVALID_FILE_ATTRIBUTES)
        .then_some(value)
        .ok_or(RecoveryCleanupError::RemovalFailed)
}

fn empty_find_data() -> FindData {
    // SAFETY: Every field accepts zero as temporary storage for FindFirstFileW
    // to initialize before this structure is observed.
    unsafe { std::mem::zeroed() }
}

fn utf16_name(value: &[u16; 260]) -> Option<String> {
    let end = value.iter().position(|unit| *unit == 0)?;
    String::from_utf16(&value[..end]).ok()
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

struct FindGuard(FindHandle);

impl Drop for FindGuard {
    fn drop(&mut self) {
        // SAFETY: The guard owns one successful FindFirstFileW handle.
        let _ = unsafe { FindClose(self.0) };
    }
}
