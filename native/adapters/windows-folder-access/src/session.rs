use std::sync::{Arc, Mutex};

use anodrel_folder_access::{
    FolderEntries, FolderEntryService, FolderEntryServiceError, FolderReference,
    FolderSelectionStore, FolderSelectionStoreError,
};

use crate::{FolderAccessError, WindowsSelectedFolder, raw::new_folder_reference};

/// Thread-safe selected-folder entry service for one authenticated Windows session.
///
/// The host registers only folders it captured from its own picker. Reading takes
/// the retained handle out of the session store first, so every reference is
/// consumed even when later native enumeration fails.
#[derive(Clone, Debug, Default)]
pub struct WindowsFolderEntryService {
    selections: Arc<Mutex<WindowsSessionFolderSelections>>,
}

impl WindowsFolderEntryService {
    /// Builds an empty service for one authenticated session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one host-captured directory under a fresh opaque reference.
    pub fn register(
        &self,
        folder: WindowsSelectedFolder,
    ) -> Result<FolderReference, SessionFolderSelectionError> {
        self.selections
            .lock()
            .map_err(|_| SessionFolderSelectionError::Unavailable)?
            .register(folder)
    }

    /// Revokes all unconsumed selected folders and closes their handles.
    pub fn clear(&self) {
        if let Ok(mut selections) = self.selections.lock() {
            selections.clear();
        }
    }
}

impl FolderEntryService for WindowsFolderEntryService {
    fn read_entries(
        &self,
        reference: &FolderReference,
    ) -> Result<FolderEntries, FolderEntryServiceError> {
        let mut folder = self
            .selections
            .lock()
            .map_err(|_| FolderEntryServiceError::Unavailable)?
            .take(reference)
            .ok_or(FolderEntryServiceError::Unavailable)?;
        folder.read_entries().map_err(|error| match error {
            FolderAccessError::Unavailable => FolderEntryServiceError::Unavailable,
        })
    }
}

/// Host-owned selected-folder state for one authenticated Windows session.
///
/// Dropping or clearing this value closes every unconsumed directory handle.
#[derive(Debug, Default)]
pub struct WindowsSessionFolderSelections {
    entries: FolderSelectionStore<WindowsSelectedFolder>,
}

impl WindowsSessionFolderSelections {
    /// Builds an empty selected-folder registry for one authenticated session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores one retained Windows directory under a new opaque reference.
    pub fn register(
        &mut self,
        folder: WindowsSelectedFolder,
    ) -> Result<FolderReference, SessionFolderSelectionError> {
        let reference =
            new_folder_reference().map_err(|_| SessionFolderSelectionError::Unavailable)?;
        match self.entries.insert(reference.clone(), folder) {
            Ok(()) => Ok(reference),
            Err(FolderSelectionStoreError::Full) => Err(SessionFolderSelectionError::Full),
            Err(FolderSelectionStoreError::DuplicateReference) => {
                Err(SessionFolderSelectionError::Unavailable)
            }
        }
    }

    /// Consumes one session-bound reference and returns its retained directory.
    #[must_use]
    pub fn take(&mut self, reference: &FolderReference) -> Option<WindowsSelectedFolder> {
        self.entries.take(reference)
    }

    /// Revokes every unconsumed reference and closes each retained directory.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of live selected-folder references.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this session has no live selected-folder references.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Safe selected-folder registration failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionFolderSelectionError {
    /// The authenticated session reached its fixed selected-folder limit.
    Full,
    /// Windows could not safely create or retain a selected-folder entry.
    Unavailable,
}

impl std::fmt::Display for SessionFolderSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("selected folder registration is unavailable")
    }
}

impl std::error::Error for SessionFolderSelectionError {}
