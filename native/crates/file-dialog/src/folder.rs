//! One bounded selected folder value.

use std::path::PathBuf;

use crate::{FileDialogInputError, MAX_SELECTED_PATH_BYTES};

/// One bounded, absolute filesystem folder selected by the user.
///
/// Constructing this value does not inspect, enumerate, read, create, or write
/// the directory. It remains a display value rather than filesystem authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedFolderPath(PathBuf);

impl SelectedFolderPath {
    /// Validates one absolute selected folder path without filesystem access.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, FileDialogInputError> {
        let path = path.into();
        if !path.is_absolute()
            || path.as_os_str().is_empty()
            || path.to_string_lossy().len() > MAX_SELECTED_PATH_BYTES
        {
            return Err(FileDialogInputError::InvalidSelectedFolderPath);
        }
        Ok(Self(path))
    }

    /// Returns the opaque selected folder path.
    #[must_use]
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SelectedFolderPath;
    use crate::FileDialogInputError;

    #[test]
    fn accepts_only_bounded_absolute_folder_paths() {
        assert!(SelectedFolderPath::new(r"C:\\Users\\Owner\\Documents").is_ok());
        assert_eq!(
            SelectedFolderPath::new("Documents"),
            Err(FileDialogInputError::InvalidSelectedFolderPath)
        );
    }
}
