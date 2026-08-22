use std::sync::{Arc, Mutex};

use anodrel_file_access::{
    FileBinaryData, FileBinaryWriteService, FileBinaryWriteServiceError, FileTextWriteService,
    FileTextWriteServiceError, SaveReference, SaveSelectionStore, SaveSelectionStoreError,
};

use crate::{
    SelectedBinaryWriteError, SelectedTextWriteError, WindowsSaveFile, new_save_reference,
};

/// Thread-safe selected-output text service for one authenticated Windows session.
///
/// The host registers only output objects it captured itself. A successful
/// request takes the object out of the session store before writing, so the
/// opaque reference cannot be replayed even when the synchronous write fails.
#[derive(Clone, Debug, Default)]
pub struct WindowsFileTextWriteService {
    selections: Arc<Mutex<WindowsSessionSaveSelections>>,
}

impl WindowsFileTextWriteService {
    /// Builds an empty service for one authenticated session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one host-captured Windows output object under a new reference.
    pub fn register(
        &self,
        file: WindowsSaveFile,
    ) -> Result<SaveReference, SessionSaveSelectionError> {
        self.selections
            .lock()
            .map_err(|_| SessionSaveSelectionError::Unavailable)?
            .register(file)
    }

    /// Revokes all remaining output selections and drops their file objects.
    pub fn clear(&self) {
        if let Ok(mut selections) = self.selections.lock() {
            selections.clear();
        }
    }

    /// Returns the separately typed binary writer for these same selections.
    ///
    /// Both writers share one store, so either operation consumes a save
    /// reference before its retained output object can be replayed.
    #[must_use]
    pub fn binary_service(&self) -> WindowsFileBinaryWriteService {
        WindowsFileBinaryWriteService {
            selections: Arc::clone(&self.selections),
        }
    }
}

/// Thread-safe selected-output binary service for one authenticated Windows session.
///
/// This service shares its retained-object registry with the text writer but
/// receives decoded bounded bytes only. It cannot decode protocol values or
/// reopen a selected path.
#[derive(Clone, Debug)]
pub struct WindowsFileBinaryWriteService {
    selections: Arc<Mutex<WindowsSessionSaveSelections>>,
}

impl FileBinaryWriteService for WindowsFileBinaryWriteService {
    fn write_binary(
        &self,
        reference: &SaveReference,
        data: &FileBinaryData,
    ) -> Result<(), FileBinaryWriteServiceError> {
        let mut file = self
            .selections
            .lock()
            .map_err(|_| FileBinaryWriteServiceError::Unavailable)?
            .take(reference)
            .ok_or(FileBinaryWriteServiceError::Unavailable)?;
        file.write_binary(data).map_err(|error| match error {
            SelectedBinaryWriteError::Unavailable => FileBinaryWriteServiceError::Unavailable,
        })
    }

    fn discard(&self, reference: &SaveReference) {
        if let Ok(mut selections) = self.selections.lock() {
            let _ = selections.take(reference);
        }
    }
}

impl FileTextWriteService for WindowsFileTextWriteService {
    fn write_text(
        &self,
        reference: &SaveReference,
        text: &str,
    ) -> Result<(), FileTextWriteServiceError> {
        let mut file = self
            .selections
            .lock()
            .map_err(|_| FileTextWriteServiceError::Unavailable)?
            .take(reference)
            .ok_or(FileTextWriteServiceError::Unavailable)?;
        file.write_text(text).map_err(|error| match error {
            SelectedTextWriteError::Unavailable => FileTextWriteServiceError::Unavailable,
            SelectedTextWriteError::TooLarge => FileTextWriteServiceError::TooLarge,
        })
    }
}

/// Host-owned selected-output state for one authenticated Windows session.
///
/// Dropping or clearing this value closes every unconsumed Windows file handle.
/// Newly created output objects remain marked for deletion until a write begins.
#[derive(Debug, Default)]
pub struct WindowsSessionSaveSelections {
    entries: SaveSelectionStore<WindowsSaveFile>,
}

impl WindowsSessionSaveSelections {
    /// Builds an empty selected-output registry for one authenticated session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores one retained Windows output object under a new opaque reference.
    pub fn register(
        &mut self,
        file: WindowsSaveFile,
    ) -> Result<SaveReference, SessionSaveSelectionError> {
        let reference = new_save_reference().map_err(|_| SessionSaveSelectionError::Unavailable)?;
        match self.entries.insert(reference.clone(), file) {
            Ok(()) => Ok(reference),
            Err(SaveSelectionStoreError::Full) => Err(SessionSaveSelectionError::Full),
            Err(SaveSelectionStoreError::DuplicateReference) => {
                Err(SessionSaveSelectionError::Unavailable)
            }
        }
    }

    /// Consumes one session-bound reference and returns its retained output object.
    #[must_use]
    pub fn take(&mut self, reference: &SaveReference) -> Option<WindowsSaveFile> {
        self.entries.take(reference)
    }

    /// Revokes every unconsumed reference and closes its retained output object.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of live selected-output references.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this session has no live selected-output references.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Safe selected-output registration failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionSaveSelectionError {
    /// The authenticated session reached its fixed output-selection limit.
    Full,
    /// Windows could not create a safe output-selection reference.
    Unavailable,
}

impl std::fmt::Display for SessionSaveSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("selected output registration is unavailable")
    }
}

impl std::error::Error for SessionSaveSelectionError {}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anodrel_file_access::{
        FileBinaryData, FileBinaryWriteService, FileTextWriteService, FileTextWriteServiceError,
    };
    use anodrel_file_dialog::SaveFilePath;

    use crate::open_save_file;

    use super::{WindowsFileTextWriteService, WindowsSessionSaveSelections};

    fn temporary_path(stem: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "anodrel-save-session-{stem}-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos(),
        ))
    }

    #[test]
    fn unused_new_output_is_deleted_when_the_session_store_clears() {
        let path = temporary_path("clear");
        let value = SaveFilePath::new(path.clone()).expect("fixture path is absolute");
        let file = open_save_file(&value).expect("fixture is captured");
        let mut selections = WindowsSessionSaveSelections::new();
        selections.register(file).expect("selection is registered");
        assert!(path.exists());
        selections.clear();
        assert!(selections.is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn service_writes_one_registered_output_only_once() {
        let path = temporary_path("write");
        let value = SaveFilePath::new(path.clone()).expect("fixture path is absolute");
        let service = WindowsFileTextWriteService::new();
        let reference = service
            .register(open_save_file(&value).expect("fixture is captured"))
            .expect("selection is registered");

        assert_eq!(service.write_text(&reference, "session text"), Ok(()));
        assert_eq!(
            service.write_text(&reference, "again"),
            Err(FileTextWriteServiceError::Unavailable)
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("fixture is readable"),
            "session text"
        );
        std::fs::remove_file(&path).expect("fixture is removed");
    }

    #[test]
    fn binary_writer_shares_one_use_references_with_the_text_writer() {
        let path = temporary_path("binary");
        let value = SaveFilePath::new(path.clone()).expect("fixture path is absolute");
        let text = WindowsFileTextWriteService::new();
        let binary = text.binary_service();
        let reference = text
            .register(open_save_file(&value).expect("fixture is captured"))
            .expect("selection is registered");
        let data =
            FileBinaryData::from_bytes(vec![0, 255, 10]).expect("bounded binary data is valid");

        assert_eq!(binary.write_binary(&reference, &data), Ok(()));
        assert_eq!(
            text.write_text(&reference, "again"),
            Err(FileTextWriteServiceError::Unavailable)
        );
        assert_eq!(
            std::fs::read(&path).expect("fixture is readable"),
            [0, 255, 10]
        );
        std::fs::remove_file(&path).expect("fixture is removed");
    }
}
