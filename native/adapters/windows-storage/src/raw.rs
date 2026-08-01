use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
    ptr,
};

type Handle = isize;
type Bool = i32;
type Dword = u32;

const INVALID_HANDLE_VALUE: Handle = -1;
const GENERIC_READ: Dword = 0x8000_0000;
const GENERIC_WRITE: Dword = 0x4000_0000;
const FILE_SHARE_READ: Dword = 0x0000_0001;
const OPEN_EXISTING: Dword = 3;
const CREATE_NEW: Dword = 1;
const FILE_ATTRIBUTE_NORMAL: Dword = 0x0000_0080;
const FILE_ATTRIBUTE_DIRECTORY: Dword = 0x0000_0010;
const FILE_ATTRIBUTE_REPARSE_POINT: Dword = 0x0000_0400;
const FILE_FLAG_OPEN_REPARSE_POINT: Dword = 0x0020_0000;
const FILE_FLAG_WRITE_THROUGH: Dword = 0x8000_0000;
const INVALID_FILE_ATTRIBUTES: Dword = u32::MAX;
const MOVEFILE_REPLACE_EXISTING: Dword = 0x0000_0001;
const MOVEFILE_WRITE_THROUGH: Dword = 0x0000_0008;
const ERROR_FILE_NOT_FOUND: Dword = 2;
const ERROR_PATH_NOT_FOUND: Dword = 3;
const ERROR_ALREADY_EXISTS: Dword = 183;

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
    fn GetFileInformationByHandle(file: Handle, information: *mut ByHandleFileInformation) -> Bool;
    fn ReadFile(
        file: Handle,
        buffer: *mut core::ffi::c_void,
        bytes_to_read: Dword,
        bytes_read: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn WriteFile(
        file: Handle,
        buffer: *const core::ffi::c_void,
        bytes_to_write: Dword,
        bytes_written: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn FlushFileBuffers(file: Handle) -> Bool;
    fn MoveFileExW(existing: *const u16, new: *const u16, flags: Dword) -> Bool;
    fn DeleteFileW(path: *const u16) -> Bool;
    fn CloseHandle(handle: Handle) -> Bool;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReadError {
    Unavailable,
    TooLarge,
}

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

pub(super) fn read_regular_file(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, ReadError> {
    let path = wide(path);
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ,
            ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return match last_error() {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(None),
            _ => Err(ReadError::Unavailable),
        };
    }
    let result = read_handle(handle, limit);
    close(handle);
    result.map(Some)
}

pub(super) fn write_complete_file(path: &Path, bytes: &[u8]) -> Result<(), ()> {
    let path = wide(path);
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
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

pub(super) fn regular_file_exists(path: &Path) -> Result<bool, ()> {
    match attributes(path)? {
        None => Ok(false),
        Some(attributes) => {
            validate_regular(attributes)?;
            Ok(true)
        }
    }
}

pub(super) fn move_file(from: &Path, to: &Path) -> Result<(), ()> {
    let from = wide(from);
    let to = wide(to);
    let moved = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 { Err(()) } else { Ok(()) }
}

pub(super) fn delete_regular_file_if_present(path: &Path) -> Result<(), ()> {
    let Some(attributes) = attributes(path)? else {
        return Ok(());
    };
    validate_regular(attributes)?;
    let path = wide(path);
    let deleted = unsafe { DeleteFileW(path.as_ptr()) };
    if deleted != 0 || matches!(last_error(), ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) {
        Ok(())
    } else {
        Err(())
    }
}

fn ensure_directory(path: &Path) -> Result<(), ()> {
    let path_wide = wide(path);
    let created = unsafe { CreateDirectoryW(path_wide.as_ptr(), ptr::null()) };
    if created == 0 && last_error() != ERROR_ALREADY_EXISTS {
        return Err(());
    }
    let Some(attributes) = attributes(path)? else {
        return Err(());
    };
    if attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
        == FILE_ATTRIBUTE_DIRECTORY
    {
        Ok(())
    } else {
        Err(())
    }
}

fn attributes(path: &Path) -> Result<Option<Dword>, ()> {
    let path = wide(path);
    let attributes = unsafe { GetFileAttributesW(path.as_ptr()) };
    if attributes != INVALID_FILE_ATTRIBUTES {
        Ok(Some(attributes))
    } else if matches!(last_error(), ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) {
        Ok(None)
    } else {
        Err(())
    }
}

fn validate_regular(attributes: Dword) -> Result<(), ()> {
    if attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) == 0 {
        Ok(())
    } else {
        Err(())
    }
}

fn read_handle(handle: Handle, limit: usize) -> Result<Vec<u8>, ReadError> {
    let mut information = std::mem::MaybeUninit::<ByHandleFileInformation>::uninit();
    if unsafe { GetFileInformationByHandle(handle, information.as_mut_ptr()) } == 0 {
        return Err(ReadError::Unavailable);
    }
    let information = unsafe { information.assume_init() };
    if validate_regular(information.attributes).is_err() {
        return Err(ReadError::Unavailable);
    }
    let declared_size =
        (u64::from(information.file_size_high) << 32) | u64::from(information.file_size_low);
    if declared_size > limit as u64 {
        return Err(ReadError::TooLarge);
    }
    let mut output = Vec::with_capacity(declared_size as usize);
    let mut buffer = [0_u8; 4096];
    loop {
        let remaining = limit.saturating_add(1).saturating_sub(output.len());
        let count = remaining.min(buffer.len());
        let mut read = 0;
        if unsafe {
            ReadFile(
                handle,
                buffer.as_mut_ptr().cast(),
                count as Dword,
                &mut read,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(ReadError::Unavailable);
        }
        if read == 0 {
            return Ok(output);
        }
        output.extend_from_slice(&buffer[..read as usize]);
        if output.len() > limit {
            return Err(ReadError::TooLarge);
        }
    }
}

fn write_handle(handle: Handle, bytes: &[u8]) -> Result<(), ()> {
    let mut offset = 0;
    while offset < bytes.len() {
        let remaining = &bytes[offset..];
        let count = remaining.len().min(u32::MAX as usize) as Dword;
        let mut written = 0;
        if unsafe {
            WriteFile(
                handle,
                remaining.as_ptr().cast(),
                count,
                &mut written,
                ptr::null_mut(),
            )
        } == 0
            || written == 0
        {
            return Err(());
        }
        offset += written as usize;
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
fn wide(path: &Path) -> Vec<u16> {
    OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
fn last_error() -> Dword {
    unsafe { GetLastError() }
}
fn close(handle: Handle) {
    unsafe {
        let _ = CloseHandle(handle);
    }
}
