#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows read-only identity capture for one selected file.
//!
//! This is an adapter-private foundation, not an application protocol service.
//! It receives only a validated picker value, retains the opened object's
//! Windows identity, and never returns a raw handle or file bytes.

mod raw;
mod session;
mod write;
mod write_session;

use anodrel_file_access::{SaveReference, SelectionReference};
use anodrel_file_dialog::{SaveFilePath, SelectedFilePath};

pub use session::{SessionSelectionError, WindowsFileTextService, WindowsSessionSelections};
pub use write_session::{
    SessionSaveSelectionError, WindowsFileTextWriteService, WindowsSessionSaveSelections,
};

/// Maximum UTF-8 bytes the first selected-file text reader returns.
pub const MAX_SELECTED_TEXT_BYTES: usize = 32 * 1024;
/// Maximum UTF-8 bytes the first selected-output text writer accepts.
pub const MAX_SELECTED_WRITE_TEXT_BYTES: usize = 8 * 1024;

/// Creates one CNG-backed opaque reference for a selected-file session entry.
pub fn new_selection_reference() -> Result<SelectionReference, FileAccessError> {
    raw::new_selection_reference().map_err(|_| FileAccessError::Unavailable)
}

/// Creates one CNG-backed opaque reference for a selected-output session entry.
pub fn new_save_reference() -> Result<SaveReference, FileAccessError> {
    SaveReference::new(raw::new_reference_value().map_err(|_| FileAccessError::Unavailable)?)
        .map_err(|_| FileAccessError::Unavailable)
}

/// Opens one selected regular file as an identity-retaining read-only object.
pub fn open_selected_file(path: &SelectedFilePath) -> Result<WindowsSelectedFile, FileAccessError> {
    raw::open_selected_file(path.as_path())
        .map(WindowsSelectedFile)
        .map_err(|_| FileAccessError::Unavailable)
}

/// Opens one selected destination as an identity-retaining output object.
///
/// Existing contents remain unchanged until [`WindowsSaveFile::write_text`] is
/// called. A new destination is marked for delete-on-abandon by the private
/// adapter before a reference can be returned.
pub fn open_save_file(path: &SaveFilePath) -> Result<WindowsSaveFile, FileAccessError> {
    write::open_save_file(path.as_path())
        .map(WindowsSaveFile)
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

    /// Reads bounded UTF-8 text from this retained Windows file object.
    pub fn read_text(&mut self) -> Result<String, SelectedTextReadError> {
        let bytes =
            raw::read_bounded(&mut self.0, MAX_SELECTED_TEXT_BYTES).map_err(
                |error| match error {
                    raw::ReadFailure::TooLarge => SelectedTextReadError::TooLarge,
                    raw::ReadFailure::Unavailable => SelectedTextReadError::Unavailable,
                },
            )?;
        String::from_utf8(bytes).map_err(|_| SelectedTextReadError::InvalidText)
    }
}

impl std::fmt::Debug for WindowsSelectedFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WindowsSelectedFile(..)")
    }
}

/// One host-retained Windows output file object.
///
/// The underlying handle closes when this value is dropped. It is intentionally
/// not exposed to applications or protocol callers.
pub struct WindowsSaveFile(write::WriteOnlyFile);

impl WindowsSaveFile {
    /// Returns the stable Windows identity captured from the opened object.
    #[must_use]
    pub fn identity(&self) -> FileIdentity {
        self.0.identity()
    }

    /// Writes one bounded UTF-8 replacement through this retained object.
    pub fn write_text(&mut self, text: &str) -> Result<(), SelectedTextWriteError> {
        if text.len() > MAX_SELECTED_WRITE_TEXT_BYTES {
            return Err(SelectedTextWriteError::TooLarge);
        }
        self.0
            .write_text(text)
            .map_err(|_| SelectedTextWriteError::Unavailable)
    }
}

impl std::fmt::Debug for WindowsSaveFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WindowsSaveFile(..)")
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

/// Safe selected-file text read failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedTextReadError {
    /// The retained file could not be read.
    Unavailable,
    /// The retained file exceeded the fixed text limit.
    TooLarge,
    /// The retained bytes were not valid UTF-8.
    InvalidText,
}

impl std::fmt::Display for SelectedTextReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("selected file text could not be read")
    }
}

impl std::error::Error for SelectedTextReadError {}

/// Safe selected-output text-write failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedTextWriteError {
    /// The retained destination could not be written.
    Unavailable,
    /// The supplied text exceeded the fixed writer limit.
    TooLarge,
}

impl std::fmt::Display for SelectedTextWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("selected output text could not be written")
    }
}

impl std::error::Error for SelectedTextWriteError {}
