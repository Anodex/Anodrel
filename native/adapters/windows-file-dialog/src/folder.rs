//! Folder-picker conversion outside the legacy file-picker binding.

mod raw;

use anodrel_file_dialog::SelectedFolderPath;

/// Opens the direct Windows folder picker and validates its one portable path.
pub(super) fn open(owner_window: isize) -> Result<Option<SelectedFolderPath>, ()> {
    raw::select_folder(owner_window)?
        .map(SelectedFolderPath::new)
        .transpose()
        .map_err(|_| ())
}
