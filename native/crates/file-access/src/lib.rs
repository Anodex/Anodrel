//! Bounded, session-owned references for future selected-file access.
//!
//! This crate performs no random generation, filesystem I/O, path processing,
//! or native-handle work. An operating-system adapter creates an unguessable
//! reference and stores its verified native file identity in this registry.
//! See `docs/FILE_ACCESS.md` and Decision 0049.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

use anodrel_file_dialog::{FileDialogFilter, SelectedFilePath};

/// Maximum live file selections for one authenticated session.
pub const MAX_SESSION_SELECTIONS: usize = 32;
/// Exact UTF-8 byte length of a Version 1 opaque selection reference.
pub const SELECTION_REFERENCE_BYTES: usize = 22;

/// An opaque Version 1 base64url reference to host-retained selected-file state.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SelectionReference(String);

impl SelectionReference {
    /// Validates one exact opaque selection reference.
    ///
    /// The adapter must derive this from 128 bits of cryptographically secure
    /// random data. Validation deliberately does not generate a value.
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

/// A bounded, one-use store of host-retained selected-file state.
///
/// `T` is intentionally selected by the native host. The portable layer never
/// needs to know whether it is a Windows handle, a file identity, or another
/// adapter-private representation.
#[derive(Debug)]
pub struct FileSelectionStore<T> {
    entries: Vec<(SelectionReference, T)>,
}

impl<T> Default for FileSelectionStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FileSelectionStore<T> {
    /// Builds an empty store for one authenticated session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts one adapter-owned selection state under a unique reference.
    pub fn insert(
        &mut self,
        reference: SelectionReference,
        state: T,
    ) -> Result<(), FileSelectionStoreError> {
        if self.entries.len() >= MAX_SESSION_SELECTIONS {
            return Err(FileSelectionStoreError::Full);
        }
        if self
            .entries
            .iter()
            .any(|(existing, _)| existing == &reference)
        {
            return Err(FileSelectionStoreError::DuplicateReference);
        }
        self.entries.push((reference, state));
        Ok(())
    }

    /// Removes and returns one selection state, permanently consuming it.
    #[must_use]
    pub fn take(&mut self, reference: &SelectionReference) -> Option<T> {
        let index = self
            .entries
            .iter()
            .position(|(existing, _)| existing == reference)?;
        Some(self.entries.swap_remove(index).1)
    }

    /// Revokes every remaining selection at session shutdown.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of live selections in this session.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this session currently has no live selections.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A safe bounded-store failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileSelectionStoreError {
    /// A session already holds the maximum number of selections.
    Full,
    /// An adapter attempted to reuse a live opaque reference.
    DuplicateReference,
}

impl fmt::Display for FileSelectionStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("file selection store rejected the entry")
    }
}

impl std::error::Error for FileSelectionStoreError {}

/// One display-safe path paired with its opaque retained-file reference.
///
/// Constructing this portable value does not open a file. A native adapter may
/// construct it only after it has captured the selected regular file's native
/// identity for the supplied reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSelection {
    path: SelectedFilePath,
    reference: SelectionReference,
}

impl FileSelection {
    /// Pairs one selected display path with its host-retained reference.
    #[must_use]
    pub fn new(path: SelectedFilePath, reference: SelectionReference) -> Self {
        Self { path, reference }
    }

    /// Returns the display-safe selected path.
    #[must_use]
    pub fn path(&self) -> &SelectedFilePath {
        &self.path
    }

    /// Returns the opaque reference that is valid only in this host session.
    #[must_use]
    pub fn reference(&self) -> &SelectionReference {
        &self.reference
    }
}

/// The bounded result from a selection-capturing host-owned picker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileSelectionResult {
    /// The host captured one selected regular file and its private identity.
    Selected(FileSelection),
    /// The user cancelled the host-owned picker.
    Cancelled,
}

/// Captures selected-file identity while completing one host-owned picker.
///
/// Implementations must not derive a selection from a caller-supplied path or
/// reopen a path returned by another picker. The Windows implementation must
/// run this work through its host UI-thread boundary.
pub trait FileSelectionService: fmt::Debug + Send {
    /// Opens one bounded picker and captures the selected file before success.
    fn open_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileSelectionResult, FileSelectionServiceError>;
}

/// A safe selection-capture service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileSelectionServiceError {
    /// The host could not show the picker or retain its selected-file identity.
    Unavailable,
}

impl fmt::Display for FileSelectionServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected file identity is unavailable")
    }
}

impl std::error::Error for FileSelectionServiceError {}

/// A safe default service for hosts without selection-time identity capture.
#[derive(Debug, Default)]
pub struct UnavailableFileSelectionService;

impl FileSelectionService for UnavailableFileSelectionService {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileSelectionResult, FileSelectionServiceError> {
        Err(FileSelectionServiceError::Unavailable)
    }
}

/// Reads bounded UTF-8 text from one session-bound selected-file reference.
///
/// Implementations must never accept a path, native handle, or caller-selected
/// filesystem scope. A missing reference is intentionally indistinguishable
/// from an unavailable host selection at this portable boundary.
pub trait FileTextService: fmt::Debug + Send {
    /// Consumes the reference's retained file object and returns bounded text.
    fn read_text(&self, reference: &SelectionReference) -> Result<String, FileTextServiceError>;
}

/// A safe selected-file text service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileTextServiceError {
    /// The selected reference was absent, expired, or could not be read.
    Unavailable,
    /// The retained file exceeded the fixed reader limit.
    TooLarge,
    /// The retained file did not contain valid UTF-8 text.
    InvalidText,
}

impl fmt::Display for FileTextServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected file text is unavailable")
    }
}

impl std::error::Error for FileTextServiceError {}

/// A safe default service for hosts that do not expose selected-file reads.
#[derive(Debug, Default)]
pub struct UnavailableFileTextService;

impl FileTextService for UnavailableFileTextService {
    fn read_text(&self, _reference: &SelectionReference) -> Result<String, FileTextServiceError> {
        Err(FileTextServiceError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FileSelection, FileSelectionResult, FileSelectionService, FileSelectionServiceError,
        FileSelectionStore, FileSelectionStoreError, FileTextService, FileTextServiceError,
        MAX_SESSION_SELECTIONS, SelectionReference, SelectionReferenceError,
        UnavailableFileSelectionService, UnavailableFileTextService,
    };
    use anodrel_file_dialog::{FileDialogFilter, SelectedFilePath};

    const FIRST: &str = "AbCdEfGhIjKlMnOpQrStUv";
    const SECOND: &str = "ZyXwVuTsRqPoNmLkJiHgFe";

    #[test]
    fn accepts_only_exact_base64url_selection_references() {
        assert!(SelectionReference::new(FIRST).is_ok());
        assert_eq!(
            SelectionReference::new("short"),
            Err(SelectionReferenceError::Invalid)
        );
        assert_eq!(
            SelectionReference::new("AbCdEfGhIjKlMnOpQrStU!"),
            Err(SelectionReferenceError::Invalid)
        );
    }

    #[test]
    fn consumes_a_selection_once_and_revokes_the_remainder() {
        let first = SelectionReference::new(FIRST).expect("reference is valid");
        let second = SelectionReference::new(SECOND).expect("reference is valid");
        let mut store = FileSelectionStore::new();
        store.insert(first.clone(), "first").expect("entry fits");
        store.insert(second.clone(), "second").expect("entry fits");

        assert_eq!(store.take(&first), Some("first"));
        assert_eq!(store.take(&first), None);
        store.clear();
        assert_eq!(store.take(&second), None);
        assert!(store.is_empty());
    }

    #[test]
    fn refuses_duplicate_and_unbounded_live_references() {
        let reference = SelectionReference::new(FIRST).expect("reference is valid");
        let mut store = FileSelectionStore::new();
        store.insert(reference.clone(), 0_u8).expect("entry fits");
        assert_eq!(
            store.insert(reference, 1_u8),
            Err(FileSelectionStoreError::DuplicateReference)
        );

        for index in 1..MAX_SESSION_SELECTIONS {
            let value = format!("{index:022}");
            let reference = SelectionReference::new(value).expect("reference is valid");
            store.insert(reference, index as u8).expect("entry fits");
        }
        let overflow =
            SelectionReference::new("0123456789012345678901").expect("reference is valid");
        assert_eq!(
            store.insert(overflow, MAX_SESSION_SELECTIONS as u8),
            Err(FileSelectionStoreError::Full)
        );
    }

    #[test]
    fn default_file_text_service_exposes_no_filesystem_authority() {
        let reference = SelectionReference::new(FIRST).expect("reference is valid");
        assert_eq!(
            UnavailableFileTextService.read_text(&reference),
            Err(FileTextServiceError::Unavailable)
        );
    }

    #[test]
    fn default_selection_service_does_not_create_file_authority() {
        let filter =
            FileDialogFilter::new("Text", vec!["txt".to_owned()]).expect("filter is valid");
        assert_eq!(
            UnavailableFileSelectionService.open_file(&[filter]),
            Err(FileSelectionServiceError::Unavailable)
        );

        let path = SelectedFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid");
        let reference = SelectionReference::new(FIRST).expect("reference is valid");
        let selection = FileSelection::new(path, reference.clone());
        assert_eq!(selection.reference(), &reference);
        assert_eq!(
            FileSelectionResult::Selected(selection).clone(),
            FileSelectionResult::Selected(FileSelection::new(
                SelectedFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid"),
                reference,
            ))
        );
    }
}
