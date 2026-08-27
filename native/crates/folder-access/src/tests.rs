//! Focused verification for selected-folder access foundations.

use std::thread;

use anodrel_file_dialog::{
    FileDialogMailbox, FileDialogRequestKind, FileDialogSelection, SelectedFolderPath,
};

use crate::{
    FolderEntries, FolderEntryService, FolderEntryServiceError, FolderFileDialogMailbox,
    FolderReference, FolderSelection, FolderSelectionResult, FolderSelectionService,
    FolderSelectionStore, FolderSelectionStoreError, MAX_SESSION_FOLDER_SELECTIONS,
    UnavailableFolderEntryService, UnavailableFolderSelectionService,
};

const FIRST: &str = "AbCdEfGhIjKlMnOpQrStUv";
const SECOND: &str = "ZyXwVuTsRqPoNmLkJiHgFe";

#[test]
fn folder_references_are_exact_and_distinct_from_file_references() {
    let reference = FolderReference::new(FIRST).expect("reference is valid");
    assert_eq!(reference.as_str(), FIRST);
    assert!(FolderReference::new("short").is_err());
    assert_ne!(
        format!("{reference:?}"),
        format!(
            "{:?}",
            anodrel_file_dialog::SelectionReference::new(FIRST).expect("file reference is valid")
        )
    );
}

#[test]
fn consumes_folder_state_once_and_bounds_the_session() {
    let first = FolderReference::new(FIRST).expect("reference is valid");
    let second = FolderReference::new(SECOND).expect("reference is valid");
    let mut store = FolderSelectionStore::new();
    store.insert(first.clone(), 1_usize).expect("entry fits");
    store.insert(second.clone(), 2).expect("entry fits");
    assert_eq!(store.take(&first), Some(1));
    assert_eq!(store.take(&first), None);

    for index in 2..=MAX_SESSION_FOLDER_SELECTIONS {
        let reference = FolderReference::new(format!("{index:022}")).expect("reference is valid");
        store.insert(reference, index).expect("entry fits");
    }
    let overflow = FolderReference::new("0123456789012345678901").expect("reference is valid");
    assert_eq!(
        store.insert(overflow, 33),
        Err(FolderSelectionStoreError::Full)
    );
    store.clear();
    assert!(store.is_empty());
    assert_eq!(store.take(&second), None);
}

#[test]
fn unavailable_defaults_never_create_folder_authority() {
    let reference = FolderReference::new(FIRST).expect("reference is valid");
    assert_eq!(
        UnavailableFolderSelectionService.open_folder(),
        Err(crate::FolderSelectionServiceError::Unavailable)
    );
    assert_eq!(
        UnavailableFolderEntryService.read_entries(&reference),
        Err(FolderEntryServiceError::Unavailable)
    );
    assert_eq!(
        FolderEntries::new(Vec::new(), true)
            .expect("empty is valid")
            .entries(),
        []
    );
}

#[test]
fn capture_service_requires_a_folder_reference_from_the_shared_mailbox() {
    let dialogs = FileDialogMailbox::new();
    let service = FolderFileDialogMailbox::new(dialogs.clone());
    let worker = thread::spawn(move || service.open_folder());
    let request = loop {
        if let Some(request) = dialogs.take() {
            break request;
        }
        thread::yield_now();
    };
    assert_eq!(
        request.kind(),
        FileDialogRequestKind::OpenFolderWithReference
    );
    let path = SelectedFolderPath::new(r"C:\\Users\\Owner\\Documents").expect("path is valid");
    let reference = FolderReference::new(FIRST).expect("reference is valid");
    assert!(dialogs.complete(
        request.id(),
        FileDialogSelection::CapturedFolder(path.clone(), reference.clone()),
    ));
    assert_eq!(
        worker.join().expect("worker did not panic"),
        Ok(FolderSelectionResult::Selected(FolderSelection::new(
            path, reference
        )))
    );
}
