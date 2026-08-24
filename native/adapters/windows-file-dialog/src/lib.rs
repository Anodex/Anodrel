#![deny(unsafe_op_in_unsafe_fn)]
//! Direct host-only Windows open-file dialog access.
mod folder;
mod raw;
use anodrel_file_dialog::{FileDialogFilter, SaveFilePath, SelectedFilePath, SelectedFolderPath};
/// Opens one host-owned Windows file picker with the supplied strict filters.
pub fn open_file(
    filters: &[FileDialogFilter],
) -> Result<Option<SelectedFilePath>, FileDialogError> {
    open_file_with_owner(0, filters)
}
/// Opens one host-owned picker attached to the supplied host window.
///
/// The owner is selected only by trusted host code; applications never pass a
/// native handle through Anodrel's protocol.
pub fn open_file_with_owner(
    owner_window: isize,
    filters: &[FileDialogFilter],
) -> Result<Option<SelectedFilePath>, FileDialogError> {
    raw::open_file(owner_window, filters).map_err(|_| FileDialogError::Unavailable)
}

/// Opens one host-owned picker and captures the selected file before returning.
///
/// The capture callback runs synchronously on the caller's host UI thread only
/// after Windows confirms one selected path and before any result reaches a
/// worker. It may retain adapter-private file identity but must not expose a
/// native handle or filesystem failure through its result.
pub fn open_file_with_owner_and_capture<T>(
    owner_window: isize,
    filters: &[FileDialogFilter],
    capture: impl FnOnce(&SelectedFilePath) -> Result<T, ()>,
) -> Result<Option<(SelectedFilePath, T)>, FileDialogError> {
    let Some(path) = open_file_with_owner(owner_window, filters)? else {
        return Ok(None);
    };
    let captured = capture(&path).map_err(|_| FileDialogError::Unavailable)?;
    Ok(Some((path, captured)))
}

/// Opens one host-owned Windows folder picker.
pub fn open_folder() -> Result<Option<SelectedFolderPath>, FileDialogError> {
    open_folder_with_owner(0)
}

/// Opens one host-owned folder picker attached to the supplied host window.
///
/// The owner is selected only by trusted host code; applications never pass a
/// native handle, initial folder, title, or native option through Anodrel's
/// protocol.
pub fn open_folder_with_owner(
    owner_window: isize,
) -> Result<Option<SelectedFolderPath>, FileDialogError> {
    folder::open(owner_window).map_err(|_| FileDialogError::Unavailable)
}

/// Opens one host-owned Windows save picker with the supplied strict filters.
///
/// A returned destination is only a user choice. This function never creates,
/// truncates, or writes a file.
pub fn save_file(filters: &[FileDialogFilter]) -> Result<Option<SaveFilePath>, FileDialogError> {
    save_file_with_owner(0, filters)
}
/// Opens one host-owned save picker attached to the supplied host window.
pub fn save_file_with_owner(
    owner_window: isize,
    filters: &[FileDialogFilter],
) -> Result<Option<SaveFilePath>, FileDialogError> {
    raw::save_file(owner_window, filters).map_err(|_| FileDialogError::Unavailable)
}

/// Opens one host-owned save picker and captures the selected output object
/// before returning.
///
/// The capture callback runs synchronously on the caller's host UI thread only
/// after Windows confirms one selected destination and before any result
/// reaches a worker. It may retain adapter-private output state but must not
/// expose a native handle or filesystem failure through its result.
pub fn save_file_with_owner_and_capture<T>(
    owner_window: isize,
    filters: &[FileDialogFilter],
    capture: impl FnOnce(&SaveFilePath) -> Result<T, ()>,
) -> Result<Option<(SaveFilePath, T)>, FileDialogError> {
    let Some(path) = save_file_with_owner(owner_window, filters)? else {
        return Ok(None);
    };
    let captured = capture(&path).map_err(|_| FileDialogError::Unavailable)?;
    Ok(Some((path, captured)))
}
/// Safe Windows file-dialog failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogError {
    /// Windows could not display or complete the dialog.
    Unavailable,
}
impl std::fmt::Display for FileDialogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Windows file dialog is unavailable")
    }
}
impl std::error::Error for FileDialogError {}
