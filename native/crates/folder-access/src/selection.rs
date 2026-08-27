//! Selection-time folder identity capture services.

use std::fmt;

use anodrel_file_dialog::{
    FileDialogMailbox, FileDialogSelection, FileDialogService, FileDialogServiceError,
    SelectedFolderPath,
};

use crate::FolderReference;

/// One display-safe folder path paired with its opaque retained-folder reference.
///
/// Constructing this portable value does not open or enumerate a folder. A
/// native adapter may construct it only after it captured the selected folder's
/// native identity for the supplied reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderSelection {
    path: SelectedFolderPath,
    reference: FolderReference,
}

impl FolderSelection {
    /// Pairs one selected display path with its host-retained reference.
    #[must_use]
    pub fn new(path: SelectedFolderPath, reference: FolderReference) -> Self {
        Self { path, reference }
    }

    /// Returns the display-safe selected folder path.
    #[must_use]
    pub fn path(&self) -> &SelectedFolderPath {
        &self.path
    }

    /// Returns the opaque reference that is valid only in this host session.
    #[must_use]
    pub fn reference(&self) -> &FolderReference {
        &self.reference
    }
}

/// The bounded result from a folder-identity-capturing host-owned picker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSelectionResult {
    /// The host captured one selected regular folder and its private identity.
    Selected(FolderSelection),
    /// The user cancelled the host-owned picker.
    Cancelled,
}

/// Captures selected-folder identity while completing one host-owned picker.
///
/// Implementations must not derive a selection from a caller-supplied path or
/// reopen a path returned by the display-only folder picker. The Windows
/// implementation must run this work through its host UI-thread boundary.
pub trait FolderSelectionService: fmt::Debug + Send {
    /// Opens one bounded folder picker and captures the selected folder before success.
    fn open_folder(&self) -> Result<FolderSelectionResult, FolderSelectionServiceError>;
}

/// A safe folder-selection-capture service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderSelectionServiceError {
    /// The host could not show the picker or retain its selected-folder identity.
    Unavailable,
}

impl fmt::Display for FolderSelectionServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected folder identity is unavailable")
    }
}

impl std::error::Error for FolderSelectionServiceError {}

/// A safe default service for hosts without selection-time folder capture.
#[derive(Debug, Default)]
pub struct UnavailableFolderSelectionService;

impl FolderSelectionService for UnavailableFolderSelectionService {
    fn open_folder(&self) -> Result<FolderSelectionResult, FolderSelectionServiceError> {
        Err(FolderSelectionServiceError::Unavailable)
    }
}

/// Adapts the shared UI-thread dialog mailbox to selected-folder identity capture.
///
/// The UI thread must complete its `OpenFolderWithReference` request with a
/// captured folder. A display-only folder path is rejected as unavailable.
#[derive(Clone, Debug)]
pub struct FolderFileDialogMailbox {
    dialogs: FileDialogMailbox,
}

impl FolderFileDialogMailbox {
    /// Binds folder selection capture to one supplied shared dialog mailbox.
    #[must_use]
    pub fn new(dialogs: FileDialogMailbox) -> Self {
        Self { dialogs }
    }
}

impl FolderSelectionService for FolderFileDialogMailbox {
    fn open_folder(&self) -> Result<FolderSelectionResult, FolderSelectionServiceError> {
        match self.dialogs.open_folder_with_reference() {
            Ok(FileDialogSelection::CapturedFolder(path, reference)) => Ok(
                FolderSelectionResult::Selected(FolderSelection::new(path, reference)),
            ),
            Ok(FileDialogSelection::Cancelled) => Ok(FolderSelectionResult::Cancelled),
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Saved(_))
            | Ok(FileDialogSelection::Folder(_))
            | Ok(FileDialogSelection::Captured(_, _))
            | Ok(FileDialogSelection::CapturedSave(_, _)) => {
                Err(FolderSelectionServiceError::Unavailable)
            }
            Err(FileDialogServiceError::Unavailable) => {
                Err(FolderSelectionServiceError::Unavailable)
            }
        }
    }
}
