//! Bounded, session-owned access to direct entries in one selected folder.
//!
//! This crate performs no random generation, filesystem I/O, path traversal,
//! or native-handle work. An operating-system adapter creates an unguessable
//! reference and retains the selected folder's verified native identity.
//! See `docs/FOLDER_ACCESS.md` and Decision 0116.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod entries;
mod read;
mod selection;
mod store;

pub use anodrel_file_dialog::{FOLDER_REFERENCE_BYTES, FolderReference, FolderReferenceError};
pub use entries::{
    FolderEntries, FolderEntriesError, FolderEntry, FolderEntryError, FolderEntryKind,
    MAX_FOLDER_ENTRIES, MAX_FOLDER_ENTRY_NAME_BYTES,
};
pub use read::{FolderEntryService, FolderEntryServiceError, UnavailableFolderEntryService};
pub use selection::{
    FolderFileDialogMailbox, FolderSelection, FolderSelectionResult, FolderSelectionService,
    FolderSelectionServiceError, UnavailableFolderSelectionService,
};
pub use store::{FolderSelectionStore, FolderSelectionStoreError, MAX_SESSION_FOLDER_SELECTIONS};

#[cfg(test)]
mod tests;
