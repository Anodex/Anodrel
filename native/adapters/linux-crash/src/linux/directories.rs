//! Descriptor-anchored Linux traversal for the host crash-record location.

use std::{
    ffi::{CString, c_char, c_int},
    fs::File,
    io,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path},
};

const O_RDONLY: c_int = 0;
const O_DIRECTORY: c_int = 0o200000;
const O_NOFOLLOW: c_int = 0o400000;
const O_CLOEXEC: c_int = 0o2000000;
const O_NONBLOCK: c_int = 0o4000;
const F_DUPFD_CLOEXEC: c_int = 1030;
const EEXIST: i32 = 17;
const ENOENT: i32 = 2;

#[link(name = "c")]
unsafe extern "C" {
    fn geteuid() -> u32;
    fn open(path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn openat(directory: c_int, path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn mkdirat(directory: c_int, path: *const c_char, mode: u32) -> c_int;
    fn fcntl(file: c_int, command: c_int, minimum: c_int) -> c_int;
}

/// One host-owned opened directory for sibling raw operations.
pub(super) struct Directory {
    file: File,
}

impl Directory {
    /// Returns the descriptor only to this adapter's direct file operations.
    pub(super) fn raw_fd(&self) -> c_int {
        self.file.as_raw_fd()
    }

    /// Duplicates the directory descriptor with close-on-exec for enumeration.
    pub(super) fn duplicate(&self) -> Result<File, DirectoryError> {
        // SAFETY: this directory descriptor remains valid for the call and the
        // fixed command asks Linux for a new close-on-exec descriptor.
        let duplicate = unsafe { fcntl(self.raw_fd(), F_DUPFD_CLOEXEC, 0) };
        if duplicate < 0 {
            return Err(DirectoryError::Unavailable);
        }
        // SAFETY: fcntl returned a newly owned descriptor exactly once.
        Ok(unsafe { File::from_raw_fd(duplicate) })
    }

    /// Flushes one directory metadata change.
    pub(super) fn sync(&self) -> Result<(), DirectoryError> {
        self.file
            .sync_all()
            .map_err(|_| DirectoryError::Unavailable)
    }

    fn open_child(&self, component: &CString) -> Result<Self, DirectoryError> {
        open_directory_at(self.raw_fd(), component)
    }

    fn create_child(&self, component: &CString) -> Result<Self, DirectoryError> {
        // SAFETY: the parent descriptor is live, component is a valid
        // NUL-terminated name, and 0700 is the fixed Anodrel directory mode.
        let created = unsafe { mkdirat(self.raw_fd(), component.as_ptr(), 0o700) };
        if created != 0 && last_status() != Some(EEXIST) {
            return Err(DirectoryError::Unavailable);
        }
        let child = self.open_child(component)?;
        if created == 0 {
            self.sync()?;
        }
        Ok(child)
    }

    fn is_private_to(&self, uid: u32) -> bool {
        self.file.metadata().is_ok_and(|metadata| {
            metadata.is_dir() && metadata.uid() == uid && metadata.mode() & 0o077 == 0
        })
    }
}

/// Closed categories from the host-directory traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DirectoryError {
    /// One required directory is absent.
    Missing,
    /// The location cannot be safely opened or created.
    Unavailable,
}

/// Opens the existing absolute account-home path without following a link.
pub(super) fn open_existing_absolute_directory(path: &Path) -> Result<Directory, DirectoryError> {
    if !path.is_absolute() {
        return Err(DirectoryError::Unavailable);
    }
    let root = c_string(Path::new("/"))?;
    let mut current = open_directory(root.as_ptr())?;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(part) => {
                current = current.open_child(&c_string(Path::new(part))?)?;
            }
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(DirectoryError::Unavailable);
            }
        }
    }
    Ok(current)
}

/// Opens or creates the fixed private host log directory below the account home.
pub(super) fn open_host_logs(
    account_home: &Path,
    relative_logs: &Path,
    private_from: usize,
) -> Result<Directory, DirectoryError> {
    let components = relative_components(relative_logs)?;
    if private_from >= components.len() {
        return Err(DirectoryError::Unavailable);
    }

    let mut current = open_existing_absolute_directory(account_home)?;
    let uid = effective_uid();
    for (index, component) in components.iter().enumerate() {
        let next = match current.open_child(component) {
            Ok(directory) => directory,
            Err(DirectoryError::Missing) => current.create_child(component)?,
            Err(error) => return Err(error),
        };
        if index >= private_from && !next.is_private_to(uid) {
            return Err(DirectoryError::Unavailable);
        }
        current = next;
    }
    Ok(current)
}

/// Returns the effective account identifier without exposing it outside raw code.
pub(super) fn effective_uid() -> u32 {
    // SAFETY: geteuid has no parameters and returns only this process's account
    // identifier, which stays inside the native adapter.
    unsafe { geteuid() }
}

fn open_directory(path: *const c_char) -> Result<Directory, DirectoryError> {
    // SAFETY: path is an owned NUL-terminated string provided by this module.
    let descriptor = unsafe {
        open(
            path,
            O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK,
            0,
        )
    };
    directory_from_open_result(descriptor)
}

fn open_directory_at(parent: c_int, component: &CString) -> Result<Directory, DirectoryError> {
    // SAFETY: parent stays open, and component is one valid NUL-terminated name.
    let descriptor = unsafe {
        openat(
            parent,
            component.as_ptr(),
            O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK,
            0,
        )
    };
    directory_from_open_result(descriptor)
}

fn directory_from_open_result(descriptor: c_int) -> Result<Directory, DirectoryError> {
    if descriptor < 0 {
        return Err(if last_status() == Some(ENOENT) {
            DirectoryError::Missing
        } else {
            DirectoryError::Unavailable
        });
    }
    // SAFETY: open or openat returned this unique non-negative descriptor.
    let file = unsafe { File::from_raw_fd(descriptor) };
    if file.metadata().is_ok_and(|metadata| metadata.is_dir()) {
        Ok(Directory { file })
    } else {
        Err(DirectoryError::Unavailable)
    }
}

fn relative_components(path: &Path) -> Result<Vec<CString>, DirectoryError> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => components.push(c_string(Path::new(part))?),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(DirectoryError::Unavailable);
            }
        }
    }
    if components.is_empty() {
        Err(DirectoryError::Unavailable)
    } else {
        Ok(components)
    }
}

fn c_string(path: &Path) -> Result<CString, DirectoryError> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| DirectoryError::Unavailable)
}

fn last_status() -> Option<i32> {
    io::Error::last_os_error().raw_os_error()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{DirectoryError, relative_components};

    #[test]
    fn the_relative_log_walk_rejects_unsafe_components() {
        assert_eq!(
            relative_components(Path::new("")),
            Err(DirectoryError::Unavailable)
        );
        assert_eq!(
            relative_components(Path::new("../logs")),
            Err(DirectoryError::Unavailable)
        );
        assert_eq!(
            relative_components(Path::new("/logs")),
            Err(DirectoryError::Unavailable)
        );
    }
}
