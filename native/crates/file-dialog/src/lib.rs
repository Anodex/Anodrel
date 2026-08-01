//! Bounded portable values for host-owned file dialogs.
//!
//! This crate has no operating-system dialog, filesystem, or protocol call.
//! Its bounded mailbox can hand one modal request to a host UI thread without
//! allowing the protocol worker to invoke a native dialog directly.
//! See `docs/FILE_DIALOGS.md` and Decisions 0044 and 0045.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod mailbox;

use std::{fmt, path::PathBuf};

pub use mailbox::{
    FileDialogMailbox, FileDialogRequest, FileDialogRequestKind, FileDialogSelection,
    FileDialogService, FileDialogServiceError,
};

/// Maximum number of extensions in one filter.
pub const MAX_FILTER_EXTENSIONS: usize = 8;
/// Maximum UTF-8 bytes in one selected file path.
pub const MAX_SELECTED_PATH_BYTES: usize = 32 * 1024;
/// Exact UTF-8 byte length of a Version 1 opaque selection reference.
pub const SELECTION_REFERENCE_BYTES: usize = 22;

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

/// One bounded, absolute save destination selected by the user.
///
/// Constructing this value neither creates nor truncates a file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveFilePath(PathBuf);

impl SaveFilePath {
    /// Validates one absolute save destination without accessing the filesystem.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, FileDialogInputError> {
        let path = path.into();
        if !path.is_absolute()
            || path.as_os_str().is_empty()
            || path.to_string_lossy().len() > MAX_SELECTED_PATH_BYTES
        {
            return Err(FileDialogInputError::InvalidSavePath);
        }
        Ok(Self(path))
    }

    /// Returns the opaque selected destination.
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
    /// The save destination was empty, relative, or exceeded its bound.
    InvalidSavePath,
}
impl fmt::Display for FileDialogInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("file dialog input is invalid")
    }
}
impl std::error::Error for FileDialogInputError {}

/// An opaque Version 1 base64url reference to host-retained selected-file state.
///
/// A host adapter derives this from 128 bits of cryptographically secure random
/// data only after it captures the selected regular file's native identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SelectionReference(String);

impl SelectionReference {
    /// Validates one exact opaque selection reference.
    ///
    /// Validation deliberately does not generate a value or access a file.
    pub fn new(value: impl Into<String>) -> Result<Self, SelectionReferenceError> {
        let value = value.into();
        if value.len() != SELECTION_REFERENCE_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(SelectionReferenceError::Invalid);
        }
        Ok(Self(value))
    }

    /// Returns the opaque reference for the protocol boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A safe failure while validating an opaque selection reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionReferenceError {
    /// The reference was not an exact Version 1 base64url value.
    Invalid,
}

impl fmt::Display for SelectionReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selection reference is invalid")
    }
}

impl std::error::Error for SelectionReferenceError {}

#[cfg(test)]
mod tests {
    use super::{
        FileDialogFilter, FileDialogInputError, SaveFilePath, SelectedFilePath, SelectionReference,
        SelectionReferenceError,
    };
    #[test]
    fn accepts_strict_filters_and_absolute_selected_paths() {
        let filter = FileDialogFilter::new("Documents", vec!["txt".to_owned(), "json".to_owned()])
            .expect("filter is valid");
        assert_eq!(filter.extensions(), ["txt", "json"]);
        assert!(SelectedFilePath::new(r"C:\Users\Owner\document.txt").is_ok());
        assert!(SaveFilePath::new(r"C:\Users\Owner\draft.txt").is_ok());
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
        assert_eq!(
            SaveFilePath::new("draft.txt"),
            Err(FileDialogInputError::InvalidSavePath)
        );
    }

    #[test]
    fn accepts_only_exact_base64url_selection_references() {
        assert!(SelectionReference::new("AbCdEfGhIjKlMnOpQrStUv").is_ok());
        assert_eq!(
            SelectionReference::new("short"),
            Err(SelectionReferenceError::Invalid)
        );
        assert_eq!(
            SelectionReference::new("AbCdEfGhIjKlMnOpQrStU!"),
            Err(SelectionReferenceError::Invalid)
        );
    }
}
