//! Public request, result, and service values for the dialog mailbox.

use std::{fmt, time::Duration};

use crate::{
    FileDialogFilter, FolderReference, SaveFilePath, SaveReference, SelectedFilePath,
    SelectedFolderPath, SelectionReference,
};

/// Maximum time a protocol worker may wait for its host UI thread to respond.
pub const FILE_DIALOG_RESPONSE_TIMEOUT: Duration = Duration::from_secs(120);

/// A host service that can open one file selected by the user.
///
/// An implementation must never treat a selected path as filesystem authority.
pub trait FileDialogService: fmt::Debug + Send {
    /// Requests one file from a host-owned picker.
    fn open_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError>;

    /// Requests one save destination from a host-owned picker.
    fn save_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }

    /// Requests one open-file selection that must carry a retained reference.
    fn open_file_with_reference(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }

    /// Requests one folder from a host-owned picker.
    fn open_folder(&self) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }

    /// Requests one folder selection that must carry a retained reference.
    fn open_folder_with_reference(&self) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }

    /// Requests one save destination that must carry a retained output reference.
    fn save_file_with_reference(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }
}

/// The outcome of a completed file-picker request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileDialogSelection {
    /// The user selected one absolute path.
    Selected(SelectedFilePath),
    /// The user selected one absolute save destination.
    Saved(SaveFilePath),
    /// The user selected one absolute filesystem folder.
    Folder(SelectedFolderPath),
    /// The UI thread captured one selected file and its opaque reference.
    Captured(SelectedFilePath, SelectionReference),
    /// The UI thread captured one selected output object and its opaque reference.
    CapturedSave(SaveFilePath, SaveReference),
    /// The UI thread captured one selected folder and its opaque reference.
    CapturedFolder(SelectedFolderPath, FolderReference),
    /// The user dismissed the host-owned picker.
    Cancelled,
}

/// The host-owned operation represented by one pending dialog request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogRequestKind {
    /// Select one existing file.
    Open,
    /// Select one save destination.
    Save,
    /// Select one existing filesystem folder.
    OpenFolder,
    /// Select one existing folder while retaining its native identity.
    OpenFolderWithReference,
    /// Select one existing file while retaining its native identity.
    OpenWithReference,
    /// Select one save destination while retaining its native output object.
    SaveWithReference,
}

/// A safe host-service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogServiceError {
    /// The host could not route the request to, or complete it on, its UI thread.
    Unavailable,
}

impl fmt::Display for FileDialogServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("file dialog is unavailable")
    }
}

impl std::error::Error for FileDialogServiceError {}

/// A bounded pending request taken exactly once by the host UI thread.
#[derive(Clone, Debug)]
pub struct FileDialogRequest {
    pub(super) id: u64,
    pub(super) kind: FileDialogRequestKind,
    pub(super) filters: Vec<FileDialogFilter>,
}

impl FileDialogRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns whether the host must show an open or a save picker.
    #[must_use]
    pub const fn kind(&self) -> FileDialogRequestKind {
        self.kind
    }

    /// Returns the strict filters requested by the authenticated application.
    #[must_use]
    pub fn filters(&self) -> &[FileDialogFilter] {
        &self.filters
    }
}
