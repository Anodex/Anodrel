//! Direct Windows Shell Link persistence for one fixed product registration.

use std::{
    ffi::{OsString, c_void},
    path::{Path, PathBuf},
    ptr,
};

use anodrel_application::{StartMenuName, is_valid_application_id};

mod com;

type Hresult = i32;

const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0010;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;
const ERROR_ALREADY_EXISTS: u32 = 183;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const MOVEFILE_REPLACE_EXISTING: u32 = 1;
const MOVEFILE_WRITE_THROUGH: u32 = 8;
const MAX_PATH_UNITS: usize = 32_767;

const FOLDERID_COMMON_PROGRAMS: Guid = Guid::new(
    0x0139_d44e,
    0x6afe,
    0x49f2,
    [0x86, 0x90, 0x3d, 0xaf, 0xca, 0xe6, 0xff, 0xb8],
);

/// A direct Shell Link operation failed without a safe useful native detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ShortcutWriteError {
    /// The common Programs directory could not be resolved safely.
    CommonProgramsUnavailable,
    /// A required filesystem path was malformed, missing, or unsafe.
    PathInvalid,
    /// The fixed Anodrel subdirectory could not be created safely.
    DirectoryCreationFailed,
    /// Windows could not allocate a private link staging file.
    TemporaryFileUnavailable,
    /// A required COM operation was unavailable.
    ComUnavailable,
    /// Windows did not persist the staged Shell Link.
    LinkSaveFailed,
    /// Windows did not replace the existing fixed Shell Link.
    LinkReplacementFailed,
    /// Windows did not remove the fixed Shell Link.
    LinkRemovalFailed,
}

/// One fixed command line for a selected product launcher.
pub(super) struct ProductLaunchArguments(String);

impl ProductLaunchArguments {
    /// Derives the only Shell Link argument sequence Anodrel product launch uses.
    pub(super) fn for_application(application_id: &str) -> Result<Self, ShortcutWriteError> {
        if !is_valid_application_id(application_id) {
            return Err(ShortcutWriteError::PathInvalid);
        }
        Ok(Self(format!("--product-launch {application_id}")))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ProductLaunchArguments {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ProductLaunchArguments(..)")
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

impl Guid {
    const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }
}

#[link(name = "Kernel32")]
unsafe extern "system" {
    fn CreateDirectoryW(path_name: *const u16, attributes: *const c_void) -> i32;
    fn DeleteFileW(file_name: *const u16) -> i32;
    fn GetFileAttributesW(path_name: *const u16) -> u32;
    fn GetLastError() -> u32;
    fn GetTempFileNameW(
        directory: *const u16,
        prefix: *const u16,
        unique: u32,
        output: *mut u16,
    ) -> u32;
    fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
}

#[link(name = "Ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(memory: *mut c_void);
}

#[link(name = "Shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(
        folder_id: *const Guid,
        flags: u32,
        token: *mut c_void,
        path: *mut *mut u16,
    ) -> Hresult;
}

/// Replaces one fixed all-users Start-menu link using already verified data.
pub(super) fn replace_common_programs_shortcut(
    launcher_path: &Path,
    package_root: &Path,
    arguments: &ProductLaunchArguments,
    start_menu_name: &StartMenuName,
) -> Result<(), ShortcutWriteError> {
    verify_normal_file(launcher_path)?;
    verify_normal_directory(package_root)?;
    let common_programs = common_programs_directory()?;
    verify_normal_directory(&common_programs)?;
    let anodrel_directory = common_programs.join("Anodrel");
    create_or_verify_normal_directory(&anodrel_directory)?;
    let link_path = anodrel_directory.join(format!("{}.lnk", start_menu_name.as_str()));
    replace_link(launcher_path, package_root, arguments, &link_path)
}

/// Removes one fixed all-users Start-menu link using already verified data.
pub(super) fn remove_common_programs_shortcut(
    start_menu_name: &StartMenuName,
) -> Result<(), ShortcutWriteError> {
    let common_programs = common_programs_directory()?;
    verify_normal_directory(&common_programs)?;
    let anodrel_directory = common_programs.join("Anodrel");
    if !existing_normal_directory(&anodrel_directory)? {
        return Ok(());
    }
    let link_path = anodrel_directory.join(format!("{}.lnk", start_menu_name.as_str()));
    remove_regular_link(&link_path)
}

fn common_programs_directory() -> Result<PathBuf, ShortcutWriteError> {
    let mut raw_path = ptr::null_mut();
    // SAFETY: the fixed folder ID, null token, and writable output slot follow
    // the documented `SHGetKnownFolderPath` contract.
    let result = unsafe {
        SHGetKnownFolderPath(&FOLDERID_COMMON_PROGRAMS, 0, ptr::null_mut(), &mut raw_path)
    };
    let path = TaskMemoryWide(raw_path);
    if !succeeded(result) {
        return Err(ShortcutWriteError::CommonProgramsUnavailable);
    }
    path.to_path()
        .ok_or(ShortcutWriteError::CommonProgramsUnavailable)
}

fn create_or_verify_normal_directory(path: &Path) -> Result<(), ShortcutWriteError> {
    let path_wide = wide_path(path)?;
    // SAFETY: the path is NUL terminated and no security attributes are supplied.
    let created = unsafe { CreateDirectoryW(path_wide.as_ptr(), ptr::null()) };
    if created == 0 {
        // SAFETY: this reads the status immediately after the CreateDirectoryW call.
        if unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
            return Err(ShortcutWriteError::DirectoryCreationFailed);
        }
    }
    verify_normal_directory(path)
}

fn replace_link(
    launcher_path: &Path,
    package_root: &Path,
    arguments: &ProductLaunchArguments,
    link_path: &Path,
) -> Result<(), ShortcutWriteError> {
    verify_absent_or_regular_file(link_path)?;
    let parent = link_path.parent().ok_or(ShortcutWriteError::PathInvalid)?;
    verify_normal_directory(parent)?;
    let temporary = TemporaryLink::create(parent)?;
    com::persist_link(
        launcher_path,
        package_root,
        arguments.as_str(),
        temporary.path(),
    )?;
    temporary.replace(link_path)
}

fn remove_regular_link(link_path: &Path) -> Result<(), ShortcutWriteError> {
    let Some(attributes) = existing_attributes(link_path)? else {
        return Ok(());
    };
    if attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(ShortcutWriteError::PathInvalid);
    }
    let path = wide_path(link_path)?;
    // SAFETY: the fixed signed link path names an existing regular file under
    // the verified Common Programs Anodrel directory.
    (unsafe { DeleteFileW(path.as_ptr()) } != 0)
        .then_some(())
        .ok_or(ShortcutWriteError::LinkRemovalFailed)
}

fn verify_normal_file(path: &Path) -> Result<(), ShortcutWriteError> {
    let attributes = attributes(path)?;
    (attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) == 0)
        .then_some(())
        .ok_or(ShortcutWriteError::PathInvalid)
}

fn verify_normal_directory(path: &Path) -> Result<(), ShortcutWriteError> {
    let attributes = attributes(path)?;
    (attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
        == FILE_ATTRIBUTE_DIRECTORY)
        .then_some(())
        .ok_or(ShortcutWriteError::PathInvalid)
}

fn existing_normal_directory(path: &Path) -> Result<bool, ShortcutWriteError> {
    let Some(attributes) = existing_attributes(path)? else {
        return Ok(false);
    };
    (attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
        == FILE_ATTRIBUTE_DIRECTORY)
        .then_some(true)
        .ok_or(ShortcutWriteError::PathInvalid)
}

fn verify_absent_or_regular_file(path: &Path) -> Result<(), ShortcutWriteError> {
    let Some(attributes) = existing_attributes(path)? else {
        return Ok(());
    };
    (attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) == 0)
        .then_some(())
        .ok_or(ShortcutWriteError::PathInvalid)
}

fn existing_attributes(path: &Path) -> Result<Option<u32>, ShortcutWriteError> {
    let path = wide_path(path)?;
    // SAFETY: the path remains NUL terminated for this read-only Windows call.
    let attributes = unsafe { GetFileAttributesW(path.as_ptr()) };
    if attributes != INVALID_FILE_ATTRIBUTES {
        return Ok(Some(attributes));
    }
    // SAFETY: this obtains the result from the immediately preceding call.
    matches!(
        unsafe { GetLastError() },
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND
    )
    .then_some(None)
    .ok_or(ShortcutWriteError::PathInvalid)
}

fn attributes(path: &Path) -> Result<u32, ShortcutWriteError> {
    let path = wide_path(path)?;
    // SAFETY: the path remains NUL terminated for this read-only Windows call.
    let attributes = unsafe { GetFileAttributesW(path.as_ptr()) };
    (attributes != INVALID_FILE_ATTRIBUTES)
        .then_some(attributes)
        .ok_or(ShortcutWriteError::PathInvalid)
}

fn wide_path(path: &Path) -> Result<Vec<u16>, ShortcutWriteError> {
    use std::os::windows::ffi::OsStrExt;

    if !path.is_absolute() {
        return Err(ShortcutWriteError::PathInvalid);
    }
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.len() >= MAX_PATH_UNITS || wide.contains(&0) {
        return Err(ShortcutWriteError::PathInvalid);
    }
    wide.push(0);
    Ok(wide)
}

fn succeeded(result: Hresult) -> bool {
    result >= 0
}

struct TaskMemoryWide(*mut u16);

impl TaskMemoryWide {
    fn to_path(&self) -> Option<PathBuf> {
        use std::os::windows::ffi::OsStringExt;

        if self.0.is_null() {
            return None;
        }
        let length = (0..MAX_PATH_UNITS).find(|index| {
            // SAFETY: Windows returned a NUL-terminated UTF-16 buffer; this
            // strict bound prevents an unbounded read if that contract fails.
            unsafe { *self.0.add(*index) == 0 }
        })?;
        // SAFETY: the bounded search found a terminator at `length`.
        let units = unsafe { std::slice::from_raw_parts(self.0, length) };
        let path = PathBuf::from(OsString::from_wide(units));
        path.is_absolute().then_some(path)
    }
}

impl Drop for TaskMemoryWide {
    fn drop(&mut self) {
        // SAFETY: Shell32 allocated this matching task-memory result. Windows
        // permits freeing a null task-memory pointer after a failed call.
        unsafe { CoTaskMemFree(self.0.cast()) };
    }
}

struct TemporaryLink {
    path: PathBuf,
    active: bool,
}

impl TemporaryLink {
    fn create(directory: &Path) -> Result<Self, ShortcutWriteError> {
        let expected_parent = directory.to_path_buf();
        let directory = wide_path(directory)?;
        let prefix = [b'A' as u16, b'N' as u16, b'R' as u16, 0];
        let mut output = [0_u16; 260];
        // SAFETY: the directory and prefix are NUL terminated, and `output`
        // is the documented MAX_PATH-sized writable result buffer.
        let result = unsafe {
            GetTempFileNameW(directory.as_ptr(), prefix.as_ptr(), 0, output.as_mut_ptr())
        };
        if result == 0 {
            return Err(ShortcutWriteError::TemporaryFileUnavailable);
        }
        let length = output
            .iter()
            .position(|unit| *unit == 0)
            .ok_or(ShortcutWriteError::TemporaryFileUnavailable)?;
        use std::os::windows::ffi::OsStringExt;

        let path = PathBuf::from(OsString::from_wide(&output[..length]));
        if !path.is_absolute() || path.parent() != Some(expected_parent.as_path()) {
            return Err(ShortcutWriteError::TemporaryFileUnavailable);
        }
        verify_normal_file(&path)?;
        Ok(Self { path, active: true })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn replace(mut self, destination: &Path) -> Result<(), ShortcutWriteError> {
        let source = wide_path(&self.path)?;
        let destination = wide_path(destination)?;
        // SAFETY: both paths are fixed siblings in the verified normal folder;
        // replacement remains local and `WRITE_THROUGH` requests persistence.
        let moved = unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved == 0 {
            return Err(ShortcutWriteError::LinkReplacementFailed);
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for TemporaryLink {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let Ok(file_attributes) = attributes(&self.path) else {
            return;
        };
        if file_attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
            return;
        }
        let Ok(path) = wide_path(&self.path) else {
            return;
        };
        // SAFETY: this is still a regular staging file Windows created in a
        // verified directory. Failure intentionally leaves it for recovery.
        unsafe { DeleteFileW(path.as_ptr()) };
    }
}

#[cfg(test)]
mod tests;
