//! Contract tests for Protocol 1.28 folder selection.

use super::support::*;
use crate::*;

#[derive(Debug)]
struct SelectingFolderDialog;

impl FileDialogService for SelectingFolderDialog {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }

    fn open_folder(&self) -> Result<FileDialogSelection, FileDialogServiceError> {
        Ok(FileDialogSelection::Folder(
            SelectedFolderPath::new(r"C:\Users\Owner\Documents").expect("test folder is absolute"),
        ))
    }
}

#[test]
fn folder_dialog_is_separately_granted_empty_and_version_gated() {
    let accepted_host = file_dialog_host(vec![Capability::DialogOpenFolder], SelectingFolderDialog);
    let accepted =
        JsonValue::parse(&accepted_host.handle_json(&request_v1_28("dialog.open_folder", r#"{}"#)))
            .expect("folder response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "status").as_string(),
        Some("selected")
    );
    assert_eq!(
        field(field(&accepted, "result"), "path").as_string(),
        Some(r"C:\Users\Owner\Documents")
    );

    let denied = JsonValue::parse(
        &file_dialog_host(vec![], SelectingFolderDialog)
            .handle_json(&request_v1_28("dialog.open_folder", r#"{}"#)),
    )
    .expect("denied folder response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let invalid = JsonValue::parse(
        &accepted_host.handle_json(&request_v1_28("dialog.open_folder", r#"{"path":"C:/"}"#)),
    )
    .expect("invalid folder response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let unsupported =
        JsonValue::parse(&accepted_host.handle_json(&request_v1_15("dialog.open_folder", r#"{}"#)))
            .expect("unsupported folder response is JSON");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}
