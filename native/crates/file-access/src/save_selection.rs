//! Save-selection capture values and their UI-thread mailbox bridge.

use std::fmt;

use anodrel_file_dialog::{
    FileDialogFilter, FileDialogMailbox, FileDialogSelection, FileDialogService,
    FileDialogServiceError, SaveFilePath,
};

use crate::SaveReference;

/// One display-safe destination paired with its opaque retained-output reference.
///
/// Constructing this portable value does not open or create a file. A native
/// adapter may construct it only after capturing the Windows output object for
/// the supplied reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveSelection {
    path: SaveFilePath,
    reference: SaveReference,
}

impl SaveSelection {
    /// Pairs one selected display destination with its host-retained reference.
    #[must_use]
    pub fn new(path: SaveFilePath, reference: SaveReference) -> Self {
        Self { path, reference }
    }

    /// Returns the display-safe selected destination.
    #[must_use]
    pub fn path(&self) -> &SaveFilePath {
        &self.path
    }

    /// Returns the opaque reference valid only in this host session.
    #[must_use]
    pub fn reference(&self) -> &SaveReference {
        &self.reference
    }
}

/// The bounded result from an output-capturing host-owned save picker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveSelectionResult {
    /// The host captured one selected output object and its private identity.
    Selected(SaveSelection),
    /// The user cancelled the host-owned picker.
    Cancelled,
}

/// Captures output-file identity while completing one host-owned save picker.
///
/// Implementations must not derive a selection from a caller-supplied path or
/// reopen a path from the legacy save picker. The Windows implementation must
/// run this work through its host UI-thread boundary.
pub trait SaveSelectionService: fmt::Debug + Send {
    /// Opens one bounded save picker and captures its output object before success.
    fn save_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, SaveSelectionServiceError>;
}

/// A safe output-selection-capture service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveSelectionServiceError {
    /// The host could not show the picker or retain its selected output object.
    Unavailable,
}

impl fmt::Display for SaveSelectionServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected output identity is unavailable")
    }
}

impl std::error::Error for SaveSelectionServiceError {}

/// A safe default service for hosts without selection-time output capture.
#[derive(Debug, Default)]
pub struct UnavailableSaveSelectionService;

impl SaveSelectionService for UnavailableSaveSelectionService {
    fn save_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, SaveSelectionServiceError> {
        Err(SaveSelectionServiceError::Unavailable)
    }
}

/// Adapts the shared UI-thread dialog mailbox to selected-output capture.
///
/// This type shares the same one-request limit as ordinary open and save
/// dialogs. The UI thread must complete its `SaveWithReference` request with a
/// captured output object; a regular selected save path is rejected.
#[derive(Clone, Debug)]
pub struct SaveFileDialogMailbox {
    dialogs: FileDialogMailbox,
}

impl SaveFileDialogMailbox {
    /// Binds output capture to one supplied shared dialog mailbox.
    #[must_use]
    pub fn new(dialogs: FileDialogMailbox) -> Self {
        Self { dialogs }
    }
}

impl SaveSelectionService for SaveFileDialogMailbox {
    fn save_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, SaveSelectionServiceError> {
        match self.dialogs.save_file_with_reference(filters) {
            Ok(FileDialogSelection::CapturedSave(path, reference)) => Ok(
                SaveSelectionResult::Selected(SaveSelection::new(path, reference)),
            ),
            Ok(FileDialogSelection::Cancelled) => Ok(SaveSelectionResult::Cancelled),
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Saved(_))
            | Ok(FileDialogSelection::Folder(_))
            | Ok(FileDialogSelection::Captured(_, _))
            | Ok(FileDialogSelection::CapturedFolder(_, _)) => {
                Err(SaveSelectionServiceError::Unavailable)
            }
            Err(FileDialogServiceError::Unavailable) => Err(SaveSelectionServiceError::Unavailable),
        }
    }
}
