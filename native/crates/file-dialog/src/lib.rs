//! Bounded portable values for host-owned file dialogs.
//!
//! This crate has no operating-system dialog, filesystem, or protocol call.
//! See `docs/FILE_DIALOGS.md` and Decision 0044.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::{fmt, path::PathBuf};

/// Maximum number of extensions in one filter.
pub const MAX_FILTER_EXTENSIONS: usize = 8;
/// Maximum UTF-8 bytes in one selected file path.
pub const MAX_SELECTED_PATH_BYTES: usize = 32 * 1024;

/// One visible filter and its allowed filename extensions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileDialogFilter {
    label: String,
    extensions: Vec<String>,
}

impl FileDialogFilter {
    /// Builds one strict ASCII file filter.
    pub fn new(
        label: impl Into<String>,
        extensions: Vec<String>,
    ) -> Result<Self, FileDialogInputError> {
        let label = label.into();
        if label.is_empty()
            || label.len() > 64
            || !label.is_ascii()
            || label.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(FileDialogInputError::InvalidLabel);
        }
        if extensions.is_empty()
            || extensions.len() > MAX_FILTER_EXTENSIONS
            || extensions.iter().any(|extension| !is_extension(extension))
        {
            return Err(FileDialogInputError::InvalidExtension);
        }
        Ok(Self { label, extensions })
    }

    /// Returns the visible filter label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Returns the validated extensions without dots or wildcards.
    #[must_use]
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }
}

/// One bounded, absolute selected file path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedFilePath(PathBuf);

impl SelectedFilePath {
    /// Validates one absolute selected path without accessing the filesystem.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, FileDialogInputError> {
        let path = path.into();
        if !path.is_absolute()
            || path.as_os_str().is_empty()
            || path.to_string_lossy().len() > MAX_SELECTED_PATH_BYTES
        {
            return Err(FileDialogInputError::InvalidSelectedPath);
        }
        Ok(Self(path))
    }
    /// Returns the opaque selected path.
    #[must_use]
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

fn is_extension(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 16
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

/// A safe portable file-dialog validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogInputError {
    /// The visible filter label was malformed or exceeded its bound.
    InvalidLabel,
    /// An extension was malformed or the filter contained too many entries.
    InvalidExtension,
    /// The selected path was empty, relative, or exceeded its bound.
    InvalidSelectedPath,
}
impl fmt::Display for FileDialogInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("file dialog input is invalid")
    }
}
impl std::error::Error for FileDialogInputError {}

#[cfg(test)]
mod tests {
    use super::{FileDialogFilter, FileDialogInputError, SelectedFilePath};
    #[test]
    fn accepts_strict_filters_and_absolute_selected_paths() {
        let filter = FileDialogFilter::new("Documents", vec!["txt".to_owned(), "json".to_owned()])
            .expect("filter is valid");
        assert_eq!(filter.extensions(), ["txt", "json"]);
        assert!(SelectedFilePath::new(r"C:\Users\Owner\document.txt").is_ok());
    }
    #[test]
    fn rejects_raw_filter_syntax_and_relative_paths() {
        assert_eq!(
            FileDialogFilter::new("All", vec!["*.txt".to_owned()]),
            Err(FileDialogInputError::InvalidExtension)
        );
        assert_eq!(
            SelectedFilePath::new("document.txt"),
            Err(FileDialogInputError::InvalidSelectedPath)
        );
    }
}
