//! File-selection and destination-dialog capability tests.

use super::super::support::*;
use crate::*;

#[test]
fn file_dialog_requires_its_own_grant_and_returns_only_cancellation_or_a_path() {
    let accepted_host = file_dialog_host(vec![Capability::DialogOpenFile], CancellingFileDialog);
    let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
        "dialog.open_file",
        r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
    )))
    .expect("dialog response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "status").as_string(),
        Some("cancelled")
    );

    let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_7(
        "dialog.open_file",
        r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
    )))
    .expect("denied dialog response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
        "dialog.open_file",
        r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
    )))
    .expect("invalid dialog response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
}

#[test]
fn save_dialog_requires_its_own_grant_and_returns_only_cancellation_or_a_destination() {
    let accepted_host = file_dialog_host(vec![Capability::DialogSaveFile], SavingFileDialog);
    let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_8(
        "dialog.save_file",
        r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
    )))
    .expect("save dialog response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "status").as_string(),
        Some("saved")
    );
    assert!(
        field(field(&accepted, "result"), "path")
            .as_string()
            .is_some_and(|path| !path.is_empty())
    );

    let denied = JsonValue::parse(
        &file_dialog_host(vec![Capability::DialogOpenFile], SavingFileDialog).handle_json(
            &request_v1_8(
                "dialog.save_file",
                r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
            ),
        ),
    )
    .expect("denied save dialog response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_8(
        "dialog.save_file",
        r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
    )))
    .expect("invalid dialog response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let unsupported = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
        "dialog.save_file",
        r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
    )))
    .expect("unsupported save dialog response is JSON");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn selection_dialog_requires_the_open_grant_and_returns_an_opaque_reference() {
    let accepted_host = file_access_host(
        vec![Capability::DialogOpenFile],
        CapturingFileDialog,
        FixedFileText(Err(FileTextServiceError::Unavailable)),
    );
    let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
        "dialog.open_file.v2",
        r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
    )))
    .expect("selection response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "selectionReference").as_string(),
        Some("AbCdEfGhIjKlMnOpQrStUv")
    );

    let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_9(
        "dialog.open_file.v2",
        r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
    )))
    .expect("denied response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
        "dialog.open_file.v2",
        r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
    )))
    .expect("invalid response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
}
