//! Bounded, session-owned references for future selected-file access.
//!
//! This crate performs no random generation, filesystem I/O, path processing,
//! or native-handle work. An operating-system adapter creates an unguessable
//! reference and stores its verified native file identity in this registry.
//! See `docs/FILE_ACCESS.md` and Decision 0049.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod binary;
mod binary_write;
mod save_selection;
mod text;

use std::fmt;

use anodrel_file_dialog::{
    FileDialogFilter, FileDialogMailbox, FileDialogSelection, FileDialogService,
    FileDialogServiceError, SelectedFilePath,
};

pub use anodrel_file_dialog::{
    SAVE_REFERENCE_BYTES, SELECTION_REFERENCE_BYTES, SaveReference, SaveReferenceError,
    SelectionReference, SelectionReferenceError,
};
pub use binary::{FileBinaryData, FileBinaryDataError, MAX_FILE_BINARY_WRITE_BYTES};
pub use binary_write::{
    FileBinaryWriteService, FileBinaryWriteServiceError, UnavailableFileBinaryWriteService,
};
pub use save_selection::{
    SaveFileDialogMailbox, SaveSelection, SaveSelectionResult, SaveSelectionService,
    SaveSelectionServiceError, UnavailableSaveSelectionService,
};
pub use text::{
    FileTextService, FileTextServiceError, FileTextWriteService, FileTextWriteServiceError,
    MAX_FILE_TEXT_WRITE_BYTES, UnavailableFileTextService, UnavailableFileTextWriteService,
};

/// Maximum live file selections for one authenticated session.
pub const MAX_SESSION_SELECTIONS: usize = 32;
/// Maximum live output selections for one authenticated session.
pub const MAX_SESSION_SAVE_SELECTIONS: usize = 32;
/// A bounded, one-use store of host-retained selected-file state.
///
/// `T` is intentionally selected by the native host. The portable layer never
/// needs to know whether it is a Windows handle, a file identity, or another
/// adapter-private representation.
#[derive(Debug)]
pub struct FileSelectionStore<T> {
    entries: Vec<(SelectionReference, T)>,
}

impl<T> Default for FileSelectionStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FileSelectionStore<T> {
    /// Builds an empty store for one authenticated session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts one adapter-owned selection state under a unique reference.
    pub fn insert(
        &mut self,
        reference: SelectionReference,
        state: T,
    ) -> Result<(), FileSelectionStoreError> {
        if self.entries.len() >= MAX_SESSION_SELECTIONS {
            return Err(FileSelectionStoreError::Full);
        }
        if self
            .entries
            .iter()
            .any(|(existing, _)| existing == &reference)
        {
            return Err(FileSelectionStoreError::DuplicateReference);
        }
        self.entries.push((reference, state));
        Ok(())
    }

    /// Removes and returns one selection state, permanently consuming it.
    #[must_use]
    pub fn take(&mut self, reference: &SelectionReference) -> Option<T> {
        let index = self
            .entries
            .iter()
            .position(|(existing, _)| existing == reference)?;
        Some(self.entries.swap_remove(index).1)
    }

    /// Revokes every remaining selection at session shutdown.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of live selections in this session.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this session currently has no live selections.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A safe bounded-store failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileSelectionStoreError {
    /// A session already holds the maximum number of selections.
    Full,
    /// An adapter attempted to reuse a live opaque reference.
    DuplicateReference,
}

impl fmt::Display for FileSelectionStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("file selection store rejected the entry")
    }
}

impl std::error::Error for FileSelectionStoreError {}

/// A bounded, one-use store of host-retained selected-output state.
///
/// This is separate from [`FileSelectionStore`] so a read reference cannot
/// become write authority through an accidentally shared store.
#[derive(Debug)]
pub struct SaveSelectionStore<T> {
    entries: Vec<(SaveReference, T)>,
}

impl<T> Default for SaveSelectionStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SaveSelectionStore<T> {
    /// Builds an empty store for one authenticated session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts one adapter-owned output state under a unique reference.
    pub fn insert(
        &mut self,
        reference: SaveReference,
        state: T,
    ) -> Result<(), SaveSelectionStoreError> {
        if self.entries.len() >= MAX_SESSION_SAVE_SELECTIONS {
            return Err(SaveSelectionStoreError::Full);
        }
        if self
            .entries
            .iter()
            .any(|(existing, _)| existing == &reference)
        {
            return Err(SaveSelectionStoreError::DuplicateReference);
        }
        self.entries.push((reference, state));
        Ok(())
    }

    /// Removes and returns one output state, permanently consuming it.
    #[must_use]
    pub fn take(&mut self, reference: &SaveReference) -> Option<T> {
        let index = self
            .entries
            .iter()
            .position(|(existing, _)| existing == reference)?;
        Some(self.entries.swap_remove(index).1)
    }

    /// Revokes every remaining output selection at session shutdown.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of live output selections in this session.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this session currently has no live output selections.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A safe bounded-store failure for one selected output object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveSelectionStoreError {
    /// A session already holds the maximum number of output selections.
    Full,
    /// An adapter attempted to reuse a live opaque output reference.
    DuplicateReference,
}

impl fmt::Display for SaveSelectionStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("save selection store rejected the entry")
    }
}

impl std::error::Error for SaveSelectionStoreError {}

/// One display-safe path paired with its opaque retained-file reference.
///
/// Constructing this portable value does not open a file. A native adapter may
/// construct it only after it has captured the selected regular file's native
/// identity for the supplied reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSelection {
    path: SelectedFilePath,
    reference: SelectionReference,
}

impl FileSelection {
    /// Pairs one selected display path with its host-retained reference.
    #[must_use]
    pub fn new(path: SelectedFilePath, reference: SelectionReference) -> Self {
        Self { path, reference }
    }

    /// Returns the display-safe selected path.
    #[must_use]
    pub fn path(&self) -> &SelectedFilePath {
        &self.path
    }

    /// Returns the opaque reference that is valid only in this host session.
    #[must_use]
    pub fn reference(&self) -> &SelectionReference {
        &self.reference
    }
}

/// The bounded result from a selection-capturing host-owned picker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileSelectionResult {
    /// The host captured one selected regular file and its private identity.
    Selected(FileSelection),
    /// The user cancelled the host-owned picker.
    Cancelled,
}

/// Captures selected-file identity while completing one host-owned picker.
///
/// Implementations must not derive a selection from a caller-supplied path or
/// reopen a path returned by another picker. The Windows implementation must
/// run this work through its host UI-thread boundary.
pub trait FileSelectionService: fmt::Debug + Send {
    /// Opens one bounded picker and captures the selected file before success.
    fn open_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileSelectionResult, FileSelectionServiceError>;
}

/// A safe selection-capture service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileSelectionServiceError {
    /// The host could not show the picker or retain its selected-file identity.
    Unavailable,
}

impl fmt::Display for FileSelectionServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected file identity is unavailable")
    }
}

impl std::error::Error for FileSelectionServiceError {}

/// A safe default service for hosts without selection-time identity capture.
#[derive(Debug, Default)]
pub struct UnavailableFileSelectionService;

impl FileSelectionService for UnavailableFileSelectionService {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileSelectionResult, FileSelectionServiceError> {
        Err(FileSelectionServiceError::Unavailable)
    }
}

/// Adapts the shared UI-thread dialog mailbox to selection-time identity capture.
///
/// This type uses the same one-request limit as ordinary open and save dialogs.
/// The UI thread must complete its `OpenWithReference` request with a captured
/// file; a regular selected path is rejected as unavailable.
#[derive(Clone, Debug)]
pub struct SelectionFileDialogMailbox {
    dialogs: FileDialogMailbox,
}

impl SelectionFileDialogMailbox {
    /// Binds selection capture to one supplied shared dialog mailbox.
    #[must_use]
    pub fn new(dialogs: FileDialogMailbox) -> Self {
        Self { dialogs }
    }
}

impl FileSelectionService for SelectionFileDialogMailbox {
    fn open_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileSelectionResult, FileSelectionServiceError> {
        match self.dialogs.open_file_with_reference(filters) {
            Ok(FileDialogSelection::Captured(path, reference)) => Ok(
                FileSelectionResult::Selected(FileSelection::new(path, reference)),
            ),
            Ok(FileDialogSelection::Cancelled) => Ok(FileSelectionResult::Cancelled),
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Saved(_))
            | Ok(FileDialogSelection::Folder(_))
            | Ok(FileDialogSelection::CapturedSave(_, _))
            | Ok(FileDialogSelection::CapturedFolder(_, _)) => {
                Err(FileSelectionServiceError::Unavailable)
            }
            Err(FileDialogServiceError::Unavailable) => Err(FileSelectionServiceError::Unavailable),
        }
    }
}

#[cfg(test)]
mod tests;
