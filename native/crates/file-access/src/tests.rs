//! Focused verification for bounded file and save selection services.

use super::{
    FileSelection, FileSelectionResult, FileSelectionService, FileSelectionServiceError,
    FileSelectionStore, FileSelectionStoreError, FileTextService, FileTextServiceError,
    MAX_SESSION_SAVE_SELECTIONS, MAX_SESSION_SELECTIONS, SaveFileDialogMailbox, SaveReference,
    SaveSelection, SaveSelectionResult, SaveSelectionService, SaveSelectionServiceError,
    SaveSelectionStore, SaveSelectionStoreError, SelectionFileDialogMailbox, SelectionReference,
    SelectionReferenceError, UnavailableFileSelectionService, UnavailableFileTextService,
    UnavailableSaveSelectionService,
};
use std::thread;

use anodrel_file_dialog::{
    FileDialogFilter, FileDialogMailbox, FileDialogRequestKind, FileDialogSelection, SaveFilePath,
    SelectedFilePath,
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
    let overflow = SelectionReference::new("0123456789012345678901").expect("reference is valid");
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
    let filter = FileDialogFilter::new("Text", vec!["txt".to_owned()]).expect("filter is valid");
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
    let filter = FileDialogFilter::new("Text", vec!["txt".to_owned()]).expect("filter is valid");
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
