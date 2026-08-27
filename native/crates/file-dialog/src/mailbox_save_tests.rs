//! Reference-capturing save-dialog mailbox verification.

use std::thread;

use super::{FileDialogMailbox, FileDialogRequestKind, FileDialogSelection, FileDialogService};
use crate::{FileDialogFilter, SaveFilePath, SaveReference};

fn filter() -> FileDialogFilter {
    FileDialogFilter::new("Text", vec!["txt".to_owned()]).expect("test filter is valid")
}

#[test]
fn captures_a_save_destination_only_for_the_save_reference_request_kind() {
    let mailbox = FileDialogMailbox::new();
    let worker = mailbox.clone();
    let waiting = thread::spawn(move || worker.save_file_with_reference(&[filter()]));
    let request = loop {
        if let Some(request) = mailbox.take() {
            break request;
        }
        thread::yield_now();
    };
    assert_eq!(request.kind(), FileDialogRequestKind::SaveWithReference);
    let path = SaveFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid");
    let reference = SaveReference::new("AbCdEfGhIjKlMnOpQrStUv").expect("reference is valid");
    assert!(mailbox.complete(
        request.id(),
        FileDialogSelection::CapturedSave(path.clone(), reference.clone()),
    ));
    assert_eq!(
        waiting.join().expect("worker did not panic"),
        Ok(FileDialogSelection::CapturedSave(path, reference))
    );
}
