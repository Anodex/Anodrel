//! Bounded direct Linux record enumeration and fixed-name file operations.

use std::{
    ffi::{CStr, CString, c_char, c_int},
    fs::File,
    io::{self, Write},
    os::{
        fd::{FromRawFd, IntoRawFd},
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

/// Maximum immediate entries examined during one host shutdown report.
pub(super) const MAX_DIRECTORY_ENTRIES: usize = 128;
/// Maximum recognized private crash-record candidates in one report.
pub(super) const MAX_RECORD_CANDIDATES: usize = 64;

#[repr(C)]
struct Dirent {
    _inode: u64,
    _offset: i64,
    _record_length: u16,
    _entry_type: u8,
    name: [c_char; 256],
}

enum DirectoryStream {}

#[link(name = "c")]
unsafe extern "C" {
    fn openat(directory: c_int, path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn unlinkat(directory: c_int, path: *const c_char, flags: c_int) -> c_int;
    fn fdopendir(descriptor: c_int) -> *mut DirectoryStream;
    fn readdir(directory: *mut DirectoryStream) -> *mut Dirent;
    fn closedir(directory: *mut DirectoryStream) -> c_int;
    fn __errno_location() -> *mut c_int;
}

/// Enumerates private regular record names without following their paths.
pub(super) fn private_record_names(directory: &Directory) -> Result<Vec<String>, ()> {
    let duplicate = directory.duplicate().map_err(|_| ())?;
    let descriptor = duplicate.into_raw_fd();
    // SAFETY: fdopendir takes ownership of the duplicated directory descriptor.
    let stream = unsafe { fdopendir(descriptor) };
    if stream.is_null() {
        // SAFETY: fdopendir failed, so the caller still owns the descriptor and
        // recreates a File solely to close it.
        drop(unsafe { File::from_raw_fd(descriptor) });
        return Err(());
    }
    let stream = DirectoryStreamGuard(stream);
    let mut examined = 0;
    let mut records = Vec::new();
    loop {
        // SAFETY: Linux exposes one writable per-thread errno location. It is
        // cleared immediately before readdir so NULL means a clean end only
        // when the following read observes zero.
        unsafe { *__errno_location() = 0 };
        // SAFETY: stream stays valid through this loop and readdir returns an
        // entry owned by the directory stream until the next call.
        let entry = unsafe { readdir(stream.0) };
        if entry.is_null() {
            // SAFETY: __errno_location remains valid for this thread.
            return if unsafe { *__errno_location() } == 0 {
                Ok(records)
            } else {
                Err(())
            };
        }
        examined += 1;
        if examined > MAX_DIRECTORY_ENTRIES {
            return Err(());
        }
        // SAFETY: Linux dirent names are NUL-terminated within d_name for each
        // entry returned by readdir.
        let name = unsafe { CStr::from_ptr((*entry).name.as_ptr()) };
        let Ok(name) = name.to_str() else {
            continue;
        };
        if !is_record_name(name) || !is_private_regular_file(directory, name) {
            continue;
        }
        records.push(name.to_owned());
        if records.len() > MAX_RECORD_CANDIDATES {
            return Err(());
        }
    }
}

/// Writes one new private record and synchronizes its contents.
pub(super) fn write_new_record(directory: &Directory, name: &str, bytes: &[u8]) -> Result<(), ()> {
    let name = c_name(name)?;
    // SAFETY: directory is live, name is a fixed NUL-terminated component, and
    // 0600 is the documented fixed record mode.
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
    // SAFETY: openat returned one unique descriptor for this new record.
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    validate_private_regular_file(&file)?;
    file.write_all(bytes).map_err(|_| ())?;
    file.sync_all().map_err(|_| ())
}

/// Deletes one still-private fixed record name, treating an absent name as clean.
pub(super) fn delete_private_record(directory: &Directory, name: &str) -> Result<bool, ()> {
    if !is_private_regular_file(directory, name) {
        return Ok(false);
    }
    let name = c_name(name)?;
    // SAFETY: directory is live and name is a fixed generated component.
    let deleted = unsafe { unlinkat(directory.raw_fd(), name.as_ptr(), 0) };
    if deleted == 0 {
        Ok(true)
    } else if io::Error::last_os_error().raw_os_error() == Some(ENOENT) {
        Ok(false)
    } else {
        Err(())
    }
}

fn is_private_regular_file(directory: &Directory, name: &str) -> bool {
    let Ok(name) = c_name(name) else {
        return false;
    };
    // SAFETY: directory is live and name is a valid one-component C string.
    let descriptor = unsafe {
        openat(
            directory.raw_fd(),
            name.as_ptr(),
            O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK,
            0,
        )
    };
    if descriptor < 0 {
        return false;
    }
    // SAFETY: openat returned one unique descriptor that this File closes.
    let file = unsafe { File::from_raw_fd(descriptor) };
    validate_private_regular_file(&file).is_ok()
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

fn is_record_name(name: &str) -> bool {
    name.strip_prefix("crash-")
        .and_then(|suffix| suffix.strip_suffix(".anodrel.v1"))
        .is_some_and(|sequence| {
            !sequence.is_empty()
                && (sequence.len() == 1 || !sequence.starts_with('0'))
                && sequence.bytes().all(|byte| byte.is_ascii_digit())
                && sequence.parse::<u64>().is_ok_and(|sequence| sequence > 0)
        })
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

struct DirectoryStreamGuard(*mut DirectoryStream);

impl Drop for DirectoryStreamGuard {
    fn drop(&mut self) {
        // SAFETY: fdopendir succeeded and transferred ownership of this stream
        // to the guard, so closedir is called exactly once.
        unsafe {
            let _ = closedir(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_record_name;

    #[test]
    fn record_names_are_canonical_bounded_decimal_file_names() {
        for name in [
            "crash-1.anodrel.v1",
            "crash-18446744073709551615.anodrel.v1",
        ] {
            assert!(is_record_name(name), "{name:?} was rejected");
        }
        for name in [
            "crash-.anodrel.v1",
            "crash-0.anodrel.v1",
            "crash-01.anodrel.v1",
            "crash--1.anodrel.v1",
            "crash-1.anodrel.v2",
            "notes.txt",
        ] {
            assert!(!is_record_name(name), "{name:?} was accepted");
        }
    }
}
