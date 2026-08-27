#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows access to one retained selected-folder handle.
//!
//! This adapter captures a folder only after the trusted Windows picker has
//! confirmed it. The later one-use enumeration reads directly from that handle:
//! it never reopens the display path or exposes a child path to an application.

mod enumerate;
mod raw;
mod session;

use anodrel_file_dialog::SelectedFolderPath;
use anodrel_folder_access::FolderEntries;

pub use session::{
    SessionFolderSelectionError, WindowsFolderEntryService, WindowsSessionFolderSelections,
};

/// Opens one picker-confirmed Windows directory as a retained folder object.
pub fn open_selected_folder(
    path: &SelectedFolderPath,
) -> Result<WindowsSelectedFolder, FolderAccessError> {
    raw::open_selected_folder(path.as_path())
        .map(WindowsSelectedFolder)
        .map_err(|_| FolderAccessError::Unavailable)
}

/// One host-retained Windows directory object.
///
/// Its handle is adapter-private and closes when this value is dropped. The
/// host may enumerate it once through a consumed reference, but applications
/// can neither receive nor select a handle or path through this type.
pub struct WindowsSelectedFolder(raw::RetainedDirectory);

impl WindowsSelectedFolder {
    /// Returns the stable Windows identity captured from the selected directory.
    #[must_use]
    pub fn identity(&self) -> FolderIdentity {
        self.0.identity()
    }

    fn read_entries(&mut self) -> Result<FolderEntries, FolderAccessError> {
        enumerate::read_entries(&mut self.0).map_err(|_| FolderAccessError::Unavailable)
    }
}

impl std::fmt::Debug for WindowsSelectedFolder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WindowsSelectedFolder(..)")
    }
}

/// A Windows volume and file-index pair for one retained selected directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FolderIdentity {
    volume_serial: u32,
    file_index: u64,
}

impl FolderIdentity {
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

/// Safe category for folder capture or enumeration failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderAccessError {
    /// Windows could not capture or enumerate the selected folder safely.
    Unavailable,
}

impl std::fmt::Display for FolderAccessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("selected folder access is unavailable")
    }
}

impl std::error::Error for FolderAccessError {}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anodrel_file_dialog::SelectedFolderPath;
    use anodrel_folder_access::{FolderEntryKind, FolderEntryService, FolderEntryServiceError};

    use super::{WindowsFolderEntryService, open_selected_folder};

    fn fixture_directory(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "anodrel-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos(),
        ))
    }

    #[test]
    fn captured_folder_enumerates_direct_entries_once_and_releases_on_consume() {
        let path = fixture_directory("folder-entries");
        std::fs::create_dir(&path).expect("fixture directory is created");
        std::fs::write(path.join("notes.txt"), "notes").expect("fixture file is written");
        std::fs::create_dir(path.join("assets")).expect("fixture child is created");
        let selected = SelectedFolderPath::new(path.clone()).expect("fixture path is absolute");
        let service = WindowsFolderEntryService::new();
        let captured = open_selected_folder(&selected).expect("folder is captured");
        assert_ne!(captured.identity().file_index(), 0);
        let reference = service.register(captured).expect("folder is registered");

        let entries = service
            .read_entries(&reference)
            .expect("folder entries are read");
        assert!(entries.is_complete());
        assert!(
            entries.entries().iter().any(|entry| {
                entry.name() == "notes.txt" && entry.kind() == FolderEntryKind::File
            })
        );
        assert!(entries.entries().iter().any(|entry| {
            entry.name() == "assets" && entry.kind() == FolderEntryKind::Directory
        }));
        assert_eq!(
            service.read_entries(&reference),
            Err(FolderEntryServiceError::Unavailable)
        );
        std::fs::remove_dir_all(&path).expect("consumed handle is released");
    }

    #[test]
    fn clear_releases_an_unconsumed_folder_handle() {
        let path = fixture_directory("folder-clear");
        std::fs::create_dir(&path).expect("fixture directory is created");
        let selected = SelectedFolderPath::new(path.clone()).expect("fixture path is absolute");
        let service = WindowsFolderEntryService::new();
        service
            .register(open_selected_folder(&selected).expect("folder is captured"))
            .expect("folder is registered");

        service.clear();
        std::fs::remove_dir_all(&path).expect("clear released the retained handle");
    }

    #[test]
    fn bounded_snapshot_reports_when_more_entries_remain() {
        let path = fixture_directory("folder-bound");
        std::fs::create_dir(&path).expect("fixture directory is created");
        for index in 0..33 {
            std::fs::write(path.join(format!("{index}.txt")), "entry")
                .expect("fixture file is written");
        }
        let selected = SelectedFolderPath::new(path.clone()).expect("fixture path is absolute");
        let service = WindowsFolderEntryService::new();
        let reference = service
            .register(open_selected_folder(&selected).expect("folder is captured"))
            .expect("folder is registered");

        let entries = service
            .read_entries(&reference)
            .expect("folder entries are read");
        assert_eq!(entries.entries().len(), 32);
        assert!(!entries.is_complete());
        std::fs::remove_dir_all(&path).expect("consumed handle is released");
    }
}
