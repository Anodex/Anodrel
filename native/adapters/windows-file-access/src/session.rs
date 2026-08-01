use anodrel_file_access::{FileSelectionStore, FileSelectionStoreError, SelectionReference};

use crate::{WindowsSelectedFile, new_selection_reference};

/// Host-owned selected-file state for one authenticated Windows session.
///
/// Dropping or clearing this value closes every unconsumed Windows file handle.
#[derive(Debug, Default)]
pub struct WindowsSessionSelections {
    entries: FileSelectionStore<WindowsSelectedFile>,
}

impl WindowsSessionSelections {
    /// Builds an empty selected-file registry for one authenticated session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores one retained Windows file object under a new opaque reference.
    pub fn register(
        &mut self,
        file: WindowsSelectedFile,
    ) -> Result<SelectionReference, SessionSelectionError> {
        let reference =
            new_selection_reference().map_err(|_| SessionSelectionError::Unavailable)?;
        match self.entries.insert(reference.clone(), file) {
            Ok(()) => Ok(reference),
            Err(FileSelectionStoreError::Full) => Err(SessionSelectionError::Full),
            Err(FileSelectionStoreError::DuplicateReference) => {
                Err(SessionSelectionError::Unavailable)
            }
        }
    }

    /// Consumes one session-bound reference and returns its retained file object.
    #[must_use]
    pub fn take(&mut self, reference: &SelectionReference) -> Option<WindowsSelectedFile> {
        self.entries.take(reference)
    }

    /// Revokes every unconsumed reference and closes its retained file object.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of live selected-file references.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether this session has no live selected-file references.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Safe selected-file registration failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionSelectionError {
    /// The authenticated session reached its fixed selection limit.
    Full,
    /// Windows could not create a safe selection reference.
    Unavailable,
}

impl std::fmt::Display for SessionSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("selected file registration is unavailable")
    }
}

impl std::error::Error for SessionSelectionError {}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anodrel_file_dialog::SelectedFilePath;

    use crate::open_selected_file;

    use super::WindowsSessionSelections;

    #[test]
    fn consumes_retained_file_identity_once_and_revokes_on_clear() {
        let path = std::env::temp_dir().join(format!(
            "anodrel-session-selection-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos(),
        ));
        std::fs::write(&path, "selected").expect("fixture is written");
        let path_value = SelectedFilePath::new(path.clone()).expect("fixture path is absolute");
        let file = open_selected_file(&path_value).expect("fixture is captured");
        let identity = file.identity();
        let mut selections = WindowsSessionSelections::new();
        let reference = selections.register(file).expect("selection is registered");

        let consumed = selections.take(&reference).expect("selection is present");
        assert_eq!(consumed.identity(), identity);
        assert!(selections.take(&reference).is_none());
        drop(consumed);

        let file = open_selected_file(&path_value).expect("fixture is captured again");
        selections
            .register(file)
            .expect("second selection is registered");
        selections.clear();
        assert!(selections.is_empty());
        std::fs::remove_file(&path).expect("clear released the retained handle");
    }
}
