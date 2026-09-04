//! Direct reparse-safe removal of one private staging tree.

use std::{
    ffi::c_void,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use super::RecoveryCleanupError;

type FindHandle = *mut c_void;

const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_NO_MORE_FILES: u32 = 18;
const MOVEFILE_DELAY_UNTIL_REBOOT: u32 = 0x0000_0004;

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
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: u32) -> i32;
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

/// Removes all normal package content except the fixed running installer image.
///
/// The root itself and its `uninstaller` child stay in place so the active
/// fixed image can continue. Every other child is removed immediately. The
/// caller may schedule the remaining image, empty child, and empty root for
/// ordered deletion after this process exits.
pub(crate) fn remove_normal_tree_except_installer(root: &Path) -> Result<(), RecoveryCleanupError> {
    let attributes = attributes(root)?;
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(RecoveryCleanupError::ReparsePointRefused);
    }
    if attributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
        return Err(RecoveryCleanupError::RemovalFailed);
    }
    let retained_directory = root.join("uninstaller");
    let retained_image = retained_directory.join("anodrel-windows-installer.exe");
    remove_children_except_installer(root, &retained_directory, &retained_image)?;
    Ok(())
}

/// Registers fixed image, directory, and root deletion for the next restart.
///
/// Windows performs delayed operations in registration order and removes a
/// delayed directory only after it is empty. This leaves no process or helper
/// running after the installation tree.
pub(crate) fn schedule_installer_tree_removal(root: &Path) -> Result<(), RecoveryCleanupError> {
    for path in scheduled_installer_removal_paths(root) {
        let wide = wide_path(&path);
        // SAFETY: Paths are derived from one preflighted normal package root;
        // the null destination and fixed flag request only delayed deletion.
        if unsafe { MoveFileExW(wide.as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT) } == 0
        {
            return Err(RecoveryCleanupError::RemovalFailed);
        }
    }
    Ok(())
}

fn scheduled_installer_removal_paths(root: &Path) -> [PathBuf; 3] {
    let directory = root.join("uninstaller");
    [
        directory.join("anodrel-windows-installer.exe"),
        directory,
        root.to_path_buf(),
    ]
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

fn remove_children_except_installer(
    root: &Path,
    retained_directory: &Path,
    retained_image: &Path,
) -> Result<(), RecoveryCleanupError> {
    let pattern = wide_path(&root.join("*"));
    let mut data = empty_find_data();
    // SAFETY: Pattern is a NUL-terminated child wildcard and data is writable.
    let handle = unsafe { FindFirstFileW(pattern.as_ptr(), &mut data) };
    if handle as isize == -1 {
        return Err(RecoveryCleanupError::RemovalFailed);
    }
    let guard = FindGuard(handle);
    let mut found_retained_directory = false;
    loop {
        let name = utf16_name(&data.file_name).ok_or(RecoveryCleanupError::RemovalFailed)?;
        if name != "." && name != ".." {
            let path = root.join(&name);
            if same_fixed_path(&path, retained_directory) {
                if data.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
                    != FILE_ATTRIBUTE_DIRECTORY
                {
                    return Err(RecoveryCleanupError::ReparsePointRefused);
                }
                found_retained_directory = true;
                remove_children_except_image(&path, retained_image)?;
            } else {
                remove_named_entry(root, &name, data.attributes)?;
            }
        }
        // SAFETY: The guard owns one live enumeration handle and data is writable.
        if unsafe { FindNextFileW(guard.0, &mut data) } == 0 {
            // SAFETY: GetLastError reads only the current thread's last-error slot.
            if unsafe { GetLastError() } != ERROR_NO_MORE_FILES {
                return Err(RecoveryCleanupError::RemovalFailed);
            }
            return found_retained_directory
                .then_some(())
                .ok_or(RecoveryCleanupError::RemovalFailed);
        }
    }
}

fn remove_children_except_image(
    directory: &Path,
    retained_image: &Path,
) -> Result<(), RecoveryCleanupError> {
    let pattern = wide_path(&directory.join("*"));
    let mut data = empty_find_data();
    // SAFETY: Pattern is a NUL-terminated child wildcard and data is writable.
    let handle = unsafe { FindFirstFileW(pattern.as_ptr(), &mut data) };
    if handle as isize == -1 {
        return Err(RecoveryCleanupError::RemovalFailed);
    }
    let guard = FindGuard(handle);
    let mut found_retained_image = false;
    loop {
        let name = utf16_name(&data.file_name).ok_or(RecoveryCleanupError::RemovalFailed)?;
        if name != "." && name != ".." {
            let path = directory.join(&name);
            if same_fixed_path(&path, retained_image) {
                if data.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
                {
                    return Err(RecoveryCleanupError::ReparsePointRefused);
                }
                found_retained_image = true;
            } else {
                remove_named_entry(directory, &name, data.attributes)?;
            }
        }
        // SAFETY: The guard owns one live enumeration handle and data is writable.
        if unsafe { FindNextFileW(guard.0, &mut data) } == 0 {
            // SAFETY: GetLastError reads only the current thread's last-error slot.
            if unsafe { GetLastError() } != ERROR_NO_MORE_FILES {
                return Err(RecoveryCleanupError::RemovalFailed);
            }
            return found_retained_image
                .then_some(())
                .ok_or(RecoveryCleanupError::RemovalFailed);
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
    remove_named_entry(root, &name, data.attributes)
}

fn remove_named_entry(
    root: &Path,
    name: &str,
    attributes: u32,
) -> Result<(), RecoveryCleanupError> {
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(RecoveryCleanupError::ReparsePointRefused);
    }
    let path = root.join(name);
    if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return remove_normal_tree(&path);
    }
    let path = wide_path(&path);
    // SAFETY: The entry came from the guarded current directory enumeration and
    // was checked not to be a directory or reparse point.
    (unsafe { DeleteFileW(path.as_ptr()) } != 0)
        .then_some(())
        .ok_or(RecoveryCleanupError::RemovalFailed)
}

fn same_fixed_path(left: &Path, right: &Path) -> bool {
    let left = left.as_os_str().to_string_lossy();
    let right = right.as_os_str().to_string_lossy();
    left.eq_ignore_ascii_case(&right)
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

#[cfg(test)]
mod tests {
    use super::scheduled_installer_removal_paths;

    #[test]
    fn delayed_removal_orders_image_before_its_empty_directories() {
        let root = std::path::Path::new("C:\\Program Files\\Anodrel\\1.2.3");
        assert_eq!(
            scheduled_installer_removal_paths(root),
            [
                root.join("uninstaller\\anodrel-windows-installer.exe"),
                root.join("uninstaller"),
                root.to_path_buf(),
            ]
        );
    }
}
