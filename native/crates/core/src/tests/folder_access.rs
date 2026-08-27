//! Contract tests for Protocol 1.29 selected-folder entry access.

use super::support::*;
use crate::*;
use anodrel_folder_access::{
    FolderEntries, FolderEntry, FolderEntryKind, FolderEntryService, FolderEntryServiceError,
    FolderReference, FolderSelection, FolderSelectionResult, FolderSelectionService,
    FolderSelectionServiceError,
};

#[derive(Debug)]
struct CapturingFolderSelection;

impl FolderSelectionService for CapturingFolderSelection {
    fn open_folder(&self) -> Result<FolderSelectionResult, FolderSelectionServiceError> {
        let path = SelectedFolderPath::new(r"C:\\Users\\Owner\\Documents")
            .expect("test folder is absolute");
        let reference = FolderReference::new("AbCdEfGhIjKlMnOpQrStUv").expect("reference is valid");
        Ok(FolderSelectionResult::Selected(FolderSelection::new(
            path, reference,
        )))
    }
}

#[derive(Debug)]
struct FixedFolderEntries;

impl FolderEntryService for FixedFolderEntries {
    fn read_entries(
        &self,
        _reference: &FolderReference,
    ) -> Result<FolderEntries, FolderEntryServiceError> {
        FolderEntries::new(
            vec![
                FolderEntry::new("notes.txt", FolderEntryKind::File).expect("entry is valid"),
                FolderEntry::new("assets", FolderEntryKind::Directory).expect("entry is valid"),
                FolderEntry::new("shortcut", FolderEntryKind::Other).expect("entry is valid"),
            ],
            false,
        )
        .map_err(|_| FolderEntryServiceError::Unavailable)
    }
}

fn folder_access_host(grants: Vec<Capability>) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        HostServices::unavailable()
            .with_folder_selections(CapturingFolderSelection)
            .with_folder_entries(FixedFolderEntries),
    )
}

#[test]
fn folder_reference_capture_and_entry_read_are_separately_granted_and_bounded() {
    let selection_host = folder_access_host(vec![Capability::DialogOpenFolder]);
    let selected = JsonValue::parse(
        &selection_host.handle_json(&request_v1_29("dialog.open_folder.v2", r#"{}"#)),
    )
    .expect("selection response is JSON");
    assert_eq!(field(&selected, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&selected, "result"), "folderReference").as_string(),
        Some("AbCdEfGhIjKlMnOpQrStUv")
    );

    let denied = JsonValue::parse(&selection_host.handle_json(&request_v1_29(
        "folder.read_entries",
        r#"{"folderReference":"AbCdEfGhIjKlMnOpQrStUv"}"#,
    )))
    .expect("denied response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let entries_host = folder_access_host(vec![Capability::FolderReadEntries]);
    let entries = JsonValue::parse(&entries_host.handle_json(&request_v1_29(
        "folder.read_entries",
        r#"{"folderReference":"AbCdEfGhIjKlMnOpQrStUv"}"#,
    )))
    .expect("entries response is JSON");
    assert_eq!(
        field(field(&entries, "result"), "status").as_string(),
        Some("entries")
    );
    let JsonValue::Array(returned_entries) = field(field(&entries, "result"), "entries") else {
        panic!("entries are an array");
    };
    assert_eq!(
        field(&returned_entries[2], "kind").as_string(),
        Some("other")
    );
    assert_eq!(
        field(field(&entries, "result"), "complete"),
        &JsonValue::Bool(false)
    );
}

#[test]
fn folder_access_rejects_invalid_payloads_and_old_protocol_versions() {
    let host = folder_access_host(vec![
        Capability::DialogOpenFolder,
        Capability::FolderReadEntries,
    ]);
    let invalid = JsonValue::parse(&host.handle_json(&request_v1_29(
        "folder.read_entries",
        r#"{"folderReference":"C:/private"}"#,
    )))
    .expect("invalid response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let unexpected = JsonValue::parse(&host.handle_json(&request_v1_29(
        "dialog.open_folder.v2",
        r#"{"title":"private"}"#,
    )))
    .expect("unexpected response is JSON");
    assert_eq!(
        field(field(&unexpected, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let old = JsonValue::parse(&host.handle_json(&request_v1_28(
        "folder.read_entries",
        r#"{"folderReference":"AbCdEfGhIjKlMnOpQrStUv"}"#,
    )))
    .expect("old response is JSON");
    assert_eq!(
        field(field(&old, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}
