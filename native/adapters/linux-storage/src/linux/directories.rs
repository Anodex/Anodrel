//! Descriptor-anchored Linux directory traversal for the state adapter.

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
const EEXIST: i32 = 17;
const ENOENT: i32 = 2;

#[link(name = "c")]
unsafe extern "C" {
    fn geteuid() -> u32;
    fn open(path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn openat(directory: c_int, path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn mkdirat(directory: c_int, path: *const c_char, mode: u32) -> c_int;
}

/// One opened Linux directory whose descriptor remains private to the adapter.
pub(super) struct Directory {
    file: File,
}

impl Directory {
    /// Returns this directory's raw descriptor only to sibling raw operations.
    pub(super) fn raw_fd(&self) -> c_int {
        self.file.as_raw_fd()
    }

    /// Flushes one metadata change in this directory.
    pub(super) fn sync(&self) -> Result<(), DirectoryError> {
        self.file
            .sync_all()
            .map_err(|_| DirectoryError::Unavailable)
    }

    fn open_child(&self, component: &CString) -> Result<Self, DirectoryError> {
        open_directory_at(self.raw_fd(), component)
    }

    fn create_child(&self, component: &CString) -> Result<Self, DirectoryError> {
        // SAFETY: the parent descriptor remains open, component is NUL
        // terminated without interior NUL bytes, and 0700 is a fixed mode.
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

/// Closed categories from descriptor traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DirectoryError {
    /// An expected directory component does not exist.
    Missing,
    /// The tree cannot be safely opened or created.
    Unavailable,
}

/// Opens one existing absolute directory, rejecting every symbolic-link step.
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
                let part = c_string(Path::new(part))?;
                current = current.open_child(&part)?;
            }
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(DirectoryError::Unavailable);
            }
        }
    }
    Ok(current)
}

/// Opens an existing data directory or creates the documented private tree.
pub(super) fn open_data_directory(
    anchor: &Path,
    relative_data_directory: &Path,
    private_from: usize,
    create: bool,
) -> Result<Option<Directory>, DirectoryError> {
    let components = relative_components(relative_data_directory)?;
    if private_from >= components.len() {
        return Err(DirectoryError::Unavailable);
    }

    let mut current = open_existing_absolute_directory(anchor)?;
    let uid = effective_uid();
    for (index, component) in components.iter().enumerate() {
        let next = match current.open_child(component) {
            Ok(directory) => directory,
            Err(DirectoryError::Missing) if create => current.create_child(component)?,
            Err(DirectoryError::Missing) => return Ok(None),
            Err(error) => return Err(error),
        };
        if index >= private_from && !next.is_private_to(uid) {
            return Err(DirectoryError::Unavailable);
        }
        current = next;
    }
    Ok(Some(current))
}

/// Returns the effective account identifier without exposing it to callers.
pub(super) fn effective_uid() -> u32 {
    // SAFETY: geteuid takes no parameters and returns this process's effective
    // Linux account ID. The value stays inside the native adapter.
    unsafe { geteuid() }
}

fn open_directory(path: *const c_char) -> Result<Directory, DirectoryError> {
    // SAFETY: path points to a valid NUL-terminated string owned by the caller.
    let file = unsafe {
        open(
            path,
            O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK,
            0,
        )
    };
    directory_from_open_result(file)
}

fn open_directory_at(parent: c_int, component: &CString) -> Result<Directory, DirectoryError> {
    // SAFETY: parent is held open by Directory, and component is a valid
    // NUL-terminated name without an embedded NUL byte.
    let file = unsafe {
        openat(
            parent,
            component.as_ptr(),
            O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK,
            0,
        )
    };
    directory_from_open_result(file)
}

fn directory_from_open_result(file: c_int) -> Result<Directory, DirectoryError> {
    if file < 0 {
        return Err(if last_status() == Some(ENOENT) {
            DirectoryError::Missing
        } else {
            DirectoryError::Unavailable
        });
    }
    // SAFETY: a non-negative descriptor was returned exclusively by open or
    // openat above, so File now owns exactly that descriptor.
    let file = unsafe { File::from_raw_fd(file) };
    if file.metadata().is_ok_and(|metadata| metadata.is_dir()) {
        Ok(Directory { file })
    } else {
        Err(DirectoryError::Unavailable)
    }
}

fn relative_components(path: &Path) -> Result<Vec<CString>, DirectoryError> {
    let mut output = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => output.push(c_string(Path::new(part))?),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(DirectoryError::Unavailable);
            }
        }
    }
    if output.is_empty() {
        Err(DirectoryError::Unavailable)
    } else {
        Ok(output)
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
    fn directory_walk_rejects_empty_and_non_relative_components() {
        assert_eq!(
            relative_components(Path::new("")),
            Err(DirectoryError::Unavailable)
        );
        assert_eq!(
            relative_components(Path::new("../state")),
            Err(DirectoryError::Unavailable)
        );
        assert_eq!(
            relative_components(Path::new("/state")),
            Err(DirectoryError::Unavailable)
        );
    }
}
