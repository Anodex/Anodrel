//! The portable entry-snapshot service seam.

use std::fmt;

use crate::{FolderEntries, FolderReference};

/// Lists one bounded direct-entry snapshot through a consumed folder reference.
///
/// Implementations receive no path, native handle, recursive flag, cursor, or
/// filesystem operation. They consume retained adapter-owned state before any
/// enumeration so the reference cannot be replayed after a later failure.
pub trait FolderEntryService: fmt::Debug + Send {
    /// Consumes one selected folder and returns its bounded direct-entry snapshot.
    fn read_entries(
        &self,
        reference: &FolderReference,
    ) -> Result<FolderEntries, FolderEntryServiceError>;
}

/// A safe selected-folder entry-snapshot failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderEntryServiceError {
    /// The reference was absent, expired, unsafe, or could not be enumerated.
    Unavailable,
}

impl fmt::Display for FolderEntryServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected folder entries are unavailable")
    }
}

impl std::error::Error for FolderEntryServiceError {}

/// A safe default service for hosts that expose no selected-folder snapshots.
#[derive(Debug, Default)]
pub struct UnavailableFolderEntryService;

impl FolderEntryService for UnavailableFolderEntryService {
    fn read_entries(
        &self,
        _reference: &FolderReference,
    ) -> Result<FolderEntries, FolderEntryServiceError> {
        Err(FolderEntryServiceError::Unavailable)
    }
}
