//! Bounded, session-owned references for future selected-file access.
//!
//! This crate performs no random generation, filesystem I/O, path processing,
//! or native-handle work. An operating-system adapter creates an unguessable
//! reference and stores its verified native file identity in this registry.
//! See `docs/FILE_ACCESS.md` and Decision 0049.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod binary;
mod binary_write;

use std::fmt;

use anodrel_file_dialog::{
    FileDialogFilter, FileDialogMailbox, FileDialogSelection, FileDialogService,
    FileDialogServiceError, SelectedFilePath,
};

pub use anodrel_file_dialog::{
    SAVE_REFERENCE_BYTES, SELECTION_REFERENCE_BYTES, SaveReference, SaveReferenceError,
    SelectionReference, SelectionReferenceError,
};
pub use binary::{FileBinaryData, FileBinaryDataError, MAX_FILE_BINARY_WRITE_BYTES};
pub use binary_write::{
    FileBinaryWriteService, FileBinaryWriteServiceError, UnavailableFileBinaryWriteService,
};

/// Maximum live file selections for one authenticated session.
pub const MAX_SESSION_SELECTIONS: usize = 32;
/// Maximum live output selections for one authenticated session.
pub const MAX_SESSION_SAVE_SELECTIONS: usize = 32;
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

/// A bounded, one-use store of host-retained selected-output state.
///
/// This is separate from [`FileSelectionStore`] so a read reference cannot
/// become write authority through an accidentally shared store.
#[derive(Debug)]
pub struct SaveSelectionStore<T> {
    entries: Vec<(SaveReference, T)>,
}

impl<T> Default for SaveSelectionStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SaveSelectionStore<T> {
    /// Builds an empty store for one authenticated session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts one adapter-owned output state under a unique reference.
    pub fn insert(
        &mut self,
        reference: SaveReference,
        state: T,
    ) -> Result<(), SaveSelectionStoreError> {
        if self.entries.len() >= MAX_SESSION_SAVE_SELECTIONS {
            return Err(SaveSelectionStoreError::Full);
        }
        if self
            .entries
            .iter()
            .any(|(existing, _)| existing == &reference)
        {
            return Err(SaveSelectionStoreError::DuplicateReference);
        }
        self.entries.push((reference, state));
        Ok(())
    }

    /// Removes and returns one output state, permanently consuming it.
    #[must_use]
    pub fn take(&mut self, reference: &SaveReference) -> Option<T> {
        let index = self
            .entries
            .iter()
            .position(|(existing, _)| existing == reference)?;
        Some(self.entries.swap_remove(index).1)
    }

    /// Revokes every remaining output selection at session shutdown.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of live output selections in this session.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this session currently has no live output selections.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A safe bounded-store failure for one selected output object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveSelectionStoreError {
    /// A session already holds the maximum number of output selections.
    Full,
    /// An adapter attempted to reuse a live opaque output reference.
    DuplicateReference,
}

impl fmt::Display for SaveSelectionStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("save selection store rejected the entry")
    }
}

impl std::error::Error for SaveSelectionStoreError {}

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

/// One display-safe destination paired with its opaque retained-output reference.
///
/// Constructing this portable value does not open or create a file. A native
/// adapter may construct it only after capturing the Windows output object for
/// the supplied reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveSelection {
    path: anodrel_file_dialog::SaveFilePath,
    reference: SaveReference,
}

impl SaveSelection {
    /// Pairs one selected display destination with its host-retained reference.
    #[must_use]
    pub fn new(path: anodrel_file_dialog::SaveFilePath, reference: SaveReference) -> Self {
        Self { path, reference }
    }

    /// Returns the display-safe selected destination.
    #[must_use]
    pub fn path(&self) -> &anodrel_file_dialog::SaveFilePath {
        &self.path
    }

    /// Returns the opaque reference valid only in this host session.
    #[must_use]
    pub fn reference(&self) -> &SaveReference {
        &self.reference
    }
}

/// The bounded result from an output-capturing host-owned save picker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveSelectionResult {
    /// The host captured one selected output object and its private identity.
    Selected(SaveSelection),
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

/// Captures output-file identity while completing one host-owned save picker.
///
/// Implementations must not derive a selection from a caller-supplied path or
/// reopen a path from the legacy save picker. The Windows implementation must
/// run this work through its host UI-thread boundary.
pub trait SaveSelectionService: fmt::Debug + Send {
    /// Opens one bounded save picker and captures its output object before success.
    fn save_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, SaveSelectionServiceError>;
}

/// A safe output-selection-capture service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveSelectionServiceError {
    /// The host could not show the picker or retain its selected output object.
    Unavailable,
}

impl fmt::Display for SaveSelectionServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected output identity is unavailable")
    }
}

impl std::error::Error for SaveSelectionServiceError {}

/// A safe default service for hosts without selection-time output capture.
#[derive(Debug, Default)]
pub struct UnavailableSaveSelectionService;

impl SaveSelectionService for UnavailableSaveSelectionService {
    fn save_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, SaveSelectionServiceError> {
        Err(SaveSelectionServiceError::Unavailable)
    }
}

/// Adapts the shared UI-thread dialog mailbox to selection-time identity capture.
///
/// This type uses the same one-request limit as ordinary open and save dialogs.
/// The UI thread must complete its `OpenWithReference` request with a captured
/// file; a regular selected path is rejected as unavailable.
#[derive(Clone, Debug)]
pub struct SelectionFileDialogMailbox {
    dialogs: FileDialogMailbox,
}

impl SelectionFileDialogMailbox {
    /// Binds selection capture to one supplied shared dialog mailbox.
    #[must_use]
    pub fn new(dialogs: FileDialogMailbox) -> Self {
        Self { dialogs }
    }
}

impl FileSelectionService for SelectionFileDialogMailbox {
    fn open_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileSelectionResult, FileSelectionServiceError> {
        match self.dialogs.open_file_with_reference(filters) {
            Ok(FileDialogSelection::Captured(path, reference)) => Ok(
                FileSelectionResult::Selected(FileSelection::new(path, reference)),
            ),
            Ok(FileDialogSelection::Cancelled) => Ok(FileSelectionResult::Cancelled),
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Saved(_))
            | Ok(FileDialogSelection::CapturedSave(_, _)) => {
                Err(FileSelectionServiceError::Unavailable)
            }
            Err(FileDialogServiceError::Unavailable) => Err(FileSelectionServiceError::Unavailable),
        }
    }
}

/// Adapts the shared UI-thread dialog mailbox to selected-output capture.
///
/// This type shares the same one-request limit as ordinary open and save
/// dialogs. The UI thread must complete its `SaveWithReference` request with a
/// captured output object; a regular selected save path is rejected.
#[derive(Clone, Debug)]
pub struct SaveFileDialogMailbox {
    dialogs: FileDialogMailbox,
}

impl SaveFileDialogMailbox {
    /// Binds output capture to one supplied shared dialog mailbox.
    #[must_use]
    pub fn new(dialogs: FileDialogMailbox) -> Self {
        Self { dialogs }
    }
}

impl SaveSelectionService for SaveFileDialogMailbox {
    fn save_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, SaveSelectionServiceError> {
        match self.dialogs.save_file_with_reference(filters) {
            Ok(FileDialogSelection::CapturedSave(path, reference)) => Ok(
                SaveSelectionResult::Selected(SaveSelection::new(path, reference)),
            ),
            Ok(FileDialogSelection::Cancelled) => Ok(SaveSelectionResult::Cancelled),
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Saved(_))
            | Ok(FileDialogSelection::Captured(_, _)) => {
                Err(SaveSelectionServiceError::Unavailable)
            }
            Err(FileDialogServiceError::Unavailable) => Err(SaveSelectionServiceError::Unavailable),
        }
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

/// Writes bounded UTF-8 text through one session-bound selected-output reference.
///
/// Implementations must never accept a path, native handle, or caller-selected
/// filesystem scope. A missing reference is intentionally indistinguishable
/// from an unavailable host output selection at this portable boundary.
pub trait FileTextWriteService: fmt::Debug + Send {
    /// Consumes the reference's retained output object and writes bounded text.
    fn write_text(
        &self,
        reference: &SaveReference,
        text: &str,
    ) -> Result<(), FileTextWriteServiceError>;
}

/// A safe selected-output text-write failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileTextWriteServiceError {
    /// The selected output reference was absent, expired, or could not be written.
    Unavailable,
    /// The supplied text exceeded the fixed writer limit.
    TooLarge,
}

impl fmt::Display for FileTextWriteServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected output text is unavailable")
    }
}

impl std::error::Error for FileTextWriteServiceError {}

/// A safe default service for hosts that do not expose selected-file writes.
#[derive(Debug, Default)]
pub struct UnavailableFileTextWriteService;

impl FileTextWriteService for UnavailableFileTextWriteService {
    fn write_text(
        &self,
        _reference: &SaveReference,
        _text: &str,
    ) -> Result<(), FileTextWriteServiceError> {
        Err(FileTextWriteServiceError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FileSelection, FileSelectionResult, FileSelectionService, FileSelectionServiceError,
        FileSelectionStore, FileSelectionStoreError, FileTextService, FileTextServiceError,
        MAX_SESSION_SAVE_SELECTIONS, MAX_SESSION_SELECTIONS, SaveFileDialogMailbox, SaveReference,
        SaveSelection, SaveSelectionResult, SaveSelectionService, SaveSelectionServiceError,
        SaveSelectionStore, SaveSelectionStoreError, SelectionFileDialogMailbox,
        SelectionReference, SelectionReferenceError, UnavailableFileSelectionService,
        UnavailableFileTextService, UnavailableSaveSelectionService,
    };
    use std::thread;

    use anodrel_file_dialog::{
        FileDialogFilter, FileDialogMailbox, FileDialogRequestKind, FileDialogSelection,
        SaveFilePath, SelectedFilePath,
    };

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
    fn consumes_output_selections_once_and_bounds_them_separately() {
        let first = SaveReference::new(FIRST).expect("reference is valid");
        let second = SaveReference::new(SECOND).expect("reference is valid");
        let mut store = SaveSelectionStore::new();
        store.insert(first.clone(), 1_u8).expect("entry fits");
        store.insert(second.clone(), 2_u8).expect("entry fits");

        assert_eq!(store.take(&first), Some(1_u8));
        assert_eq!(store.take(&first), None);
        assert_eq!(
            store.insert(second, 2_u8),
            Err(SaveSelectionStoreError::DuplicateReference)
        );
        for index in 2..=MAX_SESSION_SAVE_SELECTIONS {
            let value = format!("{index:022}");
            let reference = SaveReference::new(value).expect("reference is valid");
            store.insert(reference, index as u8).expect("entry fits");
        }
        let overflow = SaveReference::new("0123456789012345678901").expect("reference is valid");
        assert_eq!(
            store.insert(overflow, MAX_SESSION_SAVE_SELECTIONS as u8),
            Err(SaveSelectionStoreError::Full)
        );
        store.clear();
        assert!(store.is_empty());
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

    #[test]
    fn default_save_selection_service_does_not_create_file_authority() {
        let filter =
            FileDialogFilter::new("Text", vec!["txt".to_owned()]).expect("filter is valid");
        assert_eq!(
            UnavailableSaveSelectionService.save_file(&[filter]),
            Err(SaveSelectionServiceError::Unavailable)
        );

        let path = SaveFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid");
        let reference = SaveReference::new(FIRST).expect("reference is valid");
        let selection = SaveSelection::new(path, reference.clone());
        assert_eq!(selection.reference(), &reference);
        assert_eq!(
            SaveSelectionResult::Selected(selection).clone(),
            SaveSelectionResult::Selected(SaveSelection::new(
                SaveFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid"),
                reference,
            ))
        );
    }

    #[test]
    fn capture_service_uses_the_shared_dialog_mailbox_and_requires_a_reference() {
        let dialogs = FileDialogMailbox::new();
        let service = SelectionFileDialogMailbox::new(dialogs.clone());
        let worker = thread::spawn(move || {
            let filter =
                FileDialogFilter::new("Text", vec!["txt".to_owned()]).expect("filter is valid");
            service.open_file(&[filter])
        });
        let request = loop {
            if let Some(request) = dialogs.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.kind(), FileDialogRequestKind::OpenWithReference);
        let path = SelectedFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid");
        let reference = SelectionReference::new(FIRST).expect("reference is valid");
        assert!(dialogs.complete(
            request.id(),
            FileDialogSelection::Captured(path.clone(), reference.clone()),
        ));
        assert_eq!(
            worker.join().expect("worker did not panic"),
            Ok(FileSelectionResult::Selected(FileSelection::new(
                path, reference
            )))
        );
    }

    #[test]
    fn save_capture_service_uses_the_shared_dialog_mailbox_and_requires_a_reference() {
        let dialogs = FileDialogMailbox::new();
        let service = SaveFileDialogMailbox::new(dialogs.clone());
        let worker = thread::spawn(move || {
            let filter =
                FileDialogFilter::new("Text", vec!["txt".to_owned()]).expect("filter is valid");
            service.save_file(&[filter])
        });
        let request = loop {
            if let Some(request) = dialogs.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.kind(), FileDialogRequestKind::SaveWithReference);
        let path = SaveFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid");
        let reference = SaveReference::new(FIRST).expect("reference is valid");
        assert!(dialogs.complete(
            request.id(),
            FileDialogSelection::CapturedSave(path.clone(), reference.clone()),
        ));
        assert_eq!(
            worker.join().expect("worker did not panic"),
            Ok(SaveSelectionResult::Selected(SaveSelection::new(
                path, reference
            )))
        );
    }
}
