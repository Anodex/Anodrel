//! Fixed-name regular-file operations below one opened Linux data directory.

use std::{
    ffi::{CString, c_char, c_int},
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::FromRawFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::Path,
};

use super::directories::{Directory, effective_uid};

const O_RDONLY: c_int = 0;
const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 0o100;
const O_EXCL: c_int = 0o200;
const O_NONBLOCK: c_int = 0o4000;
const O_NOFOLLOW: c_int = 0o400000;
const O_CLOEXEC: c_int = 0o2000000;
const ENOENT: i32 = 2;

#[link(name = "c")]
unsafe extern "C" {
    fn openat(directory: c_int, path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn renameat(
        old_directory: c_int,
        old_path: *const c_char,
        new_directory: c_int,
        new_path: *const c_char,
    ) -> c_int;
    fn unlinkat(directory: c_int, path: *const c_char, flags: c_int) -> c_int;
}

/// Closed read categories for a state file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReadError {
    /// The opened object is not safely readable state.
    Unavailable,
    /// The object exceeds the fixed portable storage bound.
    TooLarge,
}

/// Reads one fixed regular file, treating an absent name as absent state.
pub(super) fn read_regular_file(
    directory: &Directory,
    name: &str,
    limit: usize,
) -> Result<Option<Vec<u8>>, ReadError> {
    let mut file = match open_existing_regular_file(directory, name) {
        Ok(file) => file,
        Err(RegularOpenError::Missing) => return Ok(None),
        Err(RegularOpenError::Unavailable) => return Err(ReadError::Unavailable),
    };
    let declared_size = file.metadata().map_err(|_| ReadError::Unavailable)?.len();
    if declared_size > limit as u64 {
        return Err(ReadError::TooLarge);
    }

    let mut bytes = Vec::with_capacity(declared_size as usize);
    let mut bounded = (&mut file).take(limit as u64 + 1);
    bounded
        .read_to_end(&mut bytes)
        .map_err(|_| ReadError::Unavailable)?;
    if bytes.len() > limit {
        Err(ReadError::TooLarge)
    } else {
        Ok(Some(bytes))
    }
}

/// Creates, writes, and synchronizes one fixed private staging file.
pub(super) fn write_complete_file(
    directory: &Directory,
    name: &str,
    bytes: &[u8],
) -> Result<(), ()> {
    let name = c_name(name)?;
    // SAFETY: directory remains open, name is NUL terminated without embedded
    // NUL bytes, and 0600 is the fixed state-file creation mode.
    let descriptor = unsafe {
        openat(
            directory.raw_fd(),
            name.as_ptr(),
            O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
            0o600,
        )
    };
    if descriptor < 0 {
        return Err(());
    }
    // SAFETY: openat returned a new descriptor that this File owns exactly once.
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    validate_private_regular_file(&file)?;
    file.write_all(bytes).map_err(|_| ())?;
    file.sync_all().map_err(|_| ())
}

/// Reports whether a fixed name resolves to a valid regular private state file.
pub(super) fn regular_file_exists(directory: &Directory, name: &str) -> Result<bool, ()> {
    match open_existing_regular_file(directory, name) {
        Ok(_) => Ok(true),
        Err(RegularOpenError::Missing) => Ok(false),
        Err(RegularOpenError::Unavailable) => Err(()),
    }
}

/// Renames one already validated fixed state file and synchronizes its directory.
pub(super) fn move_regular_file(directory: &Directory, from: &str, to: &str) -> Result<(), ()> {
    let _source = open_existing_regular_file(directory, from).map_err(|_| ())?;
    let from = c_name(from)?;
    let to = c_name(to)?;
    // SAFETY: the directory stays open and both names are fixed, valid,
    // NUL-terminated single path components.
    let moved = unsafe {
        renameat(
            directory.raw_fd(),
            from.as_ptr(),
            directory.raw_fd(),
            to.as_ptr(),
        )
    };
    if moved != 0 {
        return Err(());
    }
    directory.sync().map_err(|_| ())
}

/// Removes a fixed valid regular state file, returning whether it changed state.
pub(super) fn delete_regular_file_if_present(
    directory: &Directory,
    name: &str,
) -> Result<bool, ()> {
    let _file = match open_existing_regular_file(directory, name) {
        Ok(file) => file,
        Err(RegularOpenError::Missing) => return Ok(false),
        Err(RegularOpenError::Unavailable) => return Err(()),
    };
    let name = c_name(name)?;
    // SAFETY: directory stays open and name is one validated fixed component.
    let deleted = unsafe { unlinkat(directory.raw_fd(), name.as_ptr(), 0) };
    if deleted == 0 { Ok(true) } else { Err(()) }
}

fn open_existing_regular_file(directory: &Directory, name: &str) -> Result<File, RegularOpenError> {
    let name = c_name(name).map_err(|_| RegularOpenError::Unavailable)?;
    // SAFETY: directory stays open and name is a NUL-terminated fixed component.
    let descriptor = unsafe {
        openat(
            directory.raw_fd(),
            name.as_ptr(),
            O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK,
            0,
        )
    };
    if descriptor < 0 {
        return Err(
            if io::Error::last_os_error().raw_os_error() == Some(ENOENT) {
                RegularOpenError::Missing
            } else {
                RegularOpenError::Unavailable
            },
        );
    }
    // SAFETY: openat returned a new descriptor that this File owns exactly once.
    let file = unsafe { File::from_raw_fd(descriptor) };
    validate_private_regular_file(&file).map_err(|_| RegularOpenError::Unavailable)?;
    Ok(file)
}

fn validate_private_regular_file(file: &File) -> Result<(), ()> {
    let metadata = file.metadata().map_err(|_| ())?;
    if metadata.is_file()
        && metadata.nlink() == 1
        && metadata.uid() == effective_uid()
        && metadata.mode() & 0o077 == 0
    {
        Ok(())
    } else {
        Err(())
    }
}

fn c_name(name: &str) -> Result<CString, ()> {
    let path = Path::new(name);
    if path.components().count() != 1
        || !matches!(
            path.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err(());
    }
    CString::new(path.as_os_str().as_bytes()).map_err(|_| ())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegularOpenError {
    Missing,
    Unavailable,
}

#[cfg(test)]
mod tests {
    use super::c_name;

    #[test]
    fn fixed_names_reject_path_components() {
        assert!(c_name("state.anodrel.v1").is_ok());
        assert!(c_name("../state").is_err());
        assert!(c_name("state/next").is_err());
    }
}
