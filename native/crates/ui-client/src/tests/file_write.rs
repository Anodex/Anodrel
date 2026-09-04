//! Typed retained-output operation verification.

use std::path::PathBuf;

use anodrel_file_access::{FileBinaryData, MAX_FILE_TEXT_WRITE_BYTES, SaveSelectionResult};
use anodrel_file_dialog::FileDialogFilter;
use anodrel_json::JsonValue;

use super::{
    UiClientError, messages, request_field, request_protocol_minor, response,
    session_with_responses,
};

const REFERENCE: &str = "AbCdEfGhIjKlMnOpQrStUv";

#[test]
fn selected_output_uses_the_exact_protocol_1_17_operations() {
    let path = std::env::temp_dir().join("anodrel-ui-client-output.txt");
    let selected = format!(
        r#"{{"status":"selected","path":{},"saveReference":"{REFERENCE}"}}"#,
        JsonValue::String(path.to_string_lossy().into_owned()).to_json()
    );
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", &selected),
        response("anodrel-ui-2", r#"{"status":"written"}"#),
    ]);
    let filter = FileDialogFilter::new("Text documents", vec!["txt".to_owned()])
        .expect("fixed filter is valid");

    let SaveSelectionResult::Selected(selection) = session
        .select_save_file_v2(&[filter])
        .expect("host selected an output")
    else {
        panic!("the fixed response selects one output");
    };
    assert_eq!(selection.path().as_path(), PathBuf::from(&path));
    assert_eq!(selection.reference().as_str(), REFERENCE);
    session
        .write_selected_text(selection.reference(), "Hello, Anodrel.")
        .expect("host wrote retained output");

    let messages = messages(&written);
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_field(message, "operation"))
            .collect::<Vec<_>>(),
        [
            Some("dialog.save_file.v2".to_owned()),
            Some("file.write_text".to_owned()),
        ]
    );
    assert!(
        messages
            .iter()
            .skip(1)
            .all(|message| request_protocol_minor(message) == Some(17))
    );
    let dialog = JsonValue::parse(&messages[1]).expect("dialog request is JSON");
    assert_eq!(
        dialog.as_object().and_then(|fields| fields.get("payload")),
        Some(
            &JsonValue::parse(r#"{"filters":[{"label":"Text documents","extensions":["txt"]}]}"#,)
                .expect("expected filter payload is JSON"),
        )
    );
    let write = JsonValue::parse(&messages[2]).expect("write request is JSON");
    assert_eq!(
        write.as_object().and_then(|fields| fields.get("payload")),
        Some(
            &JsonValue::parse(
                r#"{"saveReference":"AbCdEfGhIjKlMnOpQrStUv","text":"Hello, Anodrel."}"#,
            )
            .expect("expected text-write payload is JSON"),
        )
    );
}

#[test]
fn cancellation_is_typed_and_malformed_results_fail_closed() {
    let (mut cancelled, _) =
        session_with_responses([response("anodrel-ui-1", r#"{"status":"cancelled"}"#)]);
    let filter = FileDialogFilter::new("Text documents", vec!["txt".to_owned()])
        .expect("fixed filter is valid");
    assert_eq!(
        cancelled.select_save_file_v2(&[filter]),
        Ok(SaveSelectionResult::Cancelled)
    );

    let (mut malformed, _) = session_with_responses([response(
        "anodrel-ui-1",
        r#"{"status":"selected","path":"relative.txt","saveReference":"AbCdEfGhIjKlMnOpQrStUv"}"#,
    )]);
    let filter = FileDialogFilter::new("Text documents", vec!["txt".to_owned()])
        .expect("fixed filter is valid");
    assert_eq!(
        malformed.select_save_file_v2(&[filter]),
        Err(UiClientError::ResponseInvalid)
    );
}

#[test]
fn invalid_filters_and_oversized_text_stop_before_a_public_request() {
    let (mut session, written) = session_with_responses([]);
    assert_eq!(
        session.select_save_file_v2(&[]),
        Err(UiClientError::FileDialogFiltersInvalid)
    );
    let reference =
        anodrel_file_access::SaveReference::new(REFERENCE).expect("fixed save reference is valid");
    assert_eq!(
        session.write_selected_text(&reference, &"x".repeat(MAX_FILE_TEXT_WRITE_BYTES + 1)),
        Err(UiClientError::FileTextInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn binary_write_uses_protocol_1_22_and_canonical_base64url() {
    let (mut session, written) = session_with_responses([
        response(
            "anodrel-ui-1",
            r#"{"status":"selected","path":"C:\\Anodrel\\output.bin","saveReference":"AbCdEfGhIjKlMnOpQrStUv"}"#,
        ),
        response("anodrel-ui-2", r#"{"status":"written"}"#),
    ]);
    let filter = FileDialogFilter::new("Binary files", vec!["bin".to_owned()])
        .expect("fixed filter is valid");
    let SaveSelectionResult::Selected(selection) = session
        .select_save_file_v2(&[filter])
        .expect("host selected an output")
    else {
        panic!("the fixed response selects one output");
    };
    let data = FileBinaryData::from_bytes(vec![0, 1, 2, 255]).expect("fixed bytes are bounded");
    session
        .write_selected_binary(selection.reference(), &data)
        .expect("host wrote retained binary output");

    let messages = messages(&written);
    assert_eq!(request_protocol_minor(&messages[1]), Some(17));
    assert_eq!(request_protocol_minor(&messages[2]), Some(22));
    assert_eq!(
        request_field(&messages[2], "operation"),
        Some("file.write_binary".to_owned())
    );
    let write = JsonValue::parse(&messages[2]).expect("binary write request is JSON");
    assert_eq!(
        write.as_object().and_then(|fields| fields.get("payload")),
        Some(
            &JsonValue::parse(
                r#"{"saveReference":"AbCdEfGhIjKlMnOpQrStUv","bytesBase64Url":"AAEC_w"}"#,
            )
            .expect("expected binary payload is JSON"),
        )
    );
}
