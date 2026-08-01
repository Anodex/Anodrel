#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows read-only identity capture for one selected file.
//!
//! This is an adapter-private foundation, not an application protocol service.
//! It receives only a validated picker value, retains the opened object's
//! Windows identity, and never returns a raw handle or file bytes.

mod raw;

use anodrel_file_access::SelectionReference;
use anodrel_file_dialog::SelectedFilePath;

/// Creates one CNG-backed opaque reference for a selected-file session entry.
pub fn new_selection_reference() -> Result<SelectionReference, FileAccessError> {
    raw::new_selection_reference().map_err(|_| FileAccessError::Unavailable)
}

/// Opens one selected regular file as an identity-retaining read-only object.
pub fn open_selected_file(path: &SelectedFilePath) -> Result<WindowsSelectedFile, FileAccessError> {
    raw::open_selected_file(path.as_path())
        .map(WindowsSelectedFile)
        .map_err(|_| FileAccessError::Unavailable)
}

/// One host-retained read-only Windows file object.
///
/// The underlying handle closes when this value is dropped. It is intentionally
/// not exposed to applications or protocol callers.
pub struct WindowsSelectedFile(raw::ReadOnlyFile);

impl WindowsSelectedFile {
    /// Returns the stable Windows identity captured from the opened object.
    #[must_use]
    pub fn identity(&self) -> FileIdentity {
        self.0.identity()
    }
}

impl std::fmt::Debug for WindowsSelectedFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WindowsSelectedFile(..)")
    }
}

/// A Windows volume and file-index pair for one opened regular file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileIdentity {
    volume_serial: u32,
    file_index: u64,
}

impl FileIdentity {
    pub(crate) const fn new(volume_serial: u32, file_index: u64) -> Self {
        Self {
            volume_serial,
            file_index,
        }
    }

    /// Returns the Windows volume serial number.
    #[must_use]
    pub const fn volume_serial(self) -> u32 {
        self.volume_serial
    }

    /// Returns the Windows file index on that volume.
    #[must_use]
    pub const fn file_index(self) -> u64 {
        self.file_index
    }
}

/// Safe category for a selected-file identity capture failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileAccessError {
    /// Windows could not open or validate the selected regular file.
    Unavailable,
}

impl std::fmt::Display for FileAccessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("selected file access is unavailable")
    }
}

impl std::error::Error for FileAccessError {}
