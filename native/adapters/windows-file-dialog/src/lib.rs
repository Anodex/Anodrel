#![deny(unsafe_op_in_unsafe_fn)]
//! Direct host-only Windows open-file dialog access.
mod raw;
use anodrel_file_dialog::{FileDialogFilter, SelectedFilePath};
/// Opens one host-owned Windows file picker with the supplied strict filters.
pub fn open_file(
    filters: &[FileDialogFilter],
) -> Result<Option<SelectedFilePath>, FileDialogError> {
    raw::open_file(filters).map_err(|_| FileDialogError::Unavailable)
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
