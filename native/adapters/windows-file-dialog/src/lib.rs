#![deny(unsafe_op_in_unsafe_fn)]
//! Direct host-only Windows open-file dialog access.
mod raw;
use anodrel_file_dialog::{FileDialogFilter, SaveFilePath, SelectedFilePath};
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
