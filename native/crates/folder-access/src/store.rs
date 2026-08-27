//! Bounded one-use stores for adapter-retained selected-folder state.

use std::fmt;

use crate::FolderReference;

/// Maximum live selected-folder references for one authenticated session.
pub const MAX_SESSION_FOLDER_SELECTIONS: usize = 32;

/// A bounded, one-use store of host-retained selected-folder state.
///
/// `T` is intentionally selected by the native adapter. The portable layer
/// never learns whether it is a Windows directory handle, identity, or another
/// adapter-private representation.
#[derive(Debug)]
pub struct FolderSelectionStore<T> {
    entries: Vec<(FolderReference, T)>,
}

impl<T> Default for FolderSelectionStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FolderSelectionStore<T> {
    /// Builds an empty store for one authenticated session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts one adapter-owned folder state under a unique reference.
    pub fn insert(
        &mut self,
        reference: FolderReference,
        state: T,
    ) -> Result<(), FolderSelectionStoreError> {
        if self.entries.len() >= MAX_SESSION_FOLDER_SELECTIONS {
            return Err(FolderSelectionStoreError::Full);
        }
        if self
            .entries
            .iter()
            .any(|(existing, _)| existing == &reference)
        {
            return Err(FolderSelectionStoreError::DuplicateReference);
        }
        self.entries.push((reference, state));
        Ok(())
    }

    /// Removes and returns one folder state, permanently consuming it.
    #[must_use]
    pub fn take(&mut self, reference: &FolderReference) -> Option<T> {
        let index = self
            .entries
            .iter()
            .position(|(existing, _)| existing == reference)?;
        Some(self.entries.swap_remove(index).1)
    }

    /// Revokes every remaining selected folder at session shutdown.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of live selected folders in this session.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this session currently has no live selected folders.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A safe bounded selected-folder store failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderSelectionStoreError {
    /// A session already holds the maximum number of selected folders.
    Full,
    /// An adapter attempted to reuse a live opaque folder reference.
    DuplicateReference,
}

impl fmt::Display for FolderSelectionStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("folder selection store rejected the entry")
    }
}

impl std::error::Error for FolderSelectionStoreError {}
