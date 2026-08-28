//! Retained save-reference write capability tests.

use super::super::support::*;
use crate::*;

#[test]
fn save_selection_requires_the_save_grant_and_returns_an_opaque_save_reference() {
    let accepted_host = file_write_host(
        vec![Capability::DialogSaveFile],
        CapturingSaveDialog,
        RecordingFileTextWrite::accepting(),
    );
    let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_17(
        "dialog.save_file.v2",
        r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
    )))
    .expect("save selection response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "saveReference").as_string(),
        Some("ZyXwVuTsRqPoNmLkJiHgFe")
    );

    let denied = JsonValue::parse(
        &file_write_host(
            vec![Capability::DialogOpenFile],
            CapturingSaveDialog,
            RecordingFileTextWrite::accepting(),
        )
        .handle_json(&request_v1_17(
            "dialog.save_file.v2",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )),
    )
    .expect("denied response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_17(
        "dialog.save_file.v2",
        r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
    )))
    .expect("invalid response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let unsupported = JsonValue::parse(&accepted_host.handle_json(&request_v1_16(
        "dialog.save_file.v2",
        r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
    )))
    .expect("unsupported response is JSON");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn selected_output_text_is_separately_granted_bounded_and_safe() {
    let writer = RecordingFileTextWrite::accepting();
    let writes = Arc::clone(&writer.writes);
    let write_host = file_write_host(vec![Capability::FileWriteText], CapturingSaveDialog, writer);
    let reference = "ZyXwVuTsRqPoNmLkJiHgFe";
    let accepted = JsonValue::parse(&write_host.handle_json(&request_v1_17(
        "file.write_text",
        &format!(r#"{{"saveReference":"{reference}","text":"selected text"}}"#),
    )))
    .expect("write response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "status").as_string(),
        Some("written")
    );
    assert_eq!(
        writes
            .lock()
            .expect("write recorder lock is available")
            .as_slice(),
        ["selected text"]
    );

    let denied = JsonValue::parse(
        &file_write_host(
            vec![Capability::DialogSaveFile],
            CapturingSaveDialog,
            RecordingFileTextWrite::accepting(),
        )
        .handle_json(&request_v1_17(
            "file.write_text",
            &format!(r#"{{"saveReference":"{reference}","text":"selected text"}}"#),
        )),
    )
    .expect("denied response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let invalid = JsonValue::parse(&write_host.handle_json(&request_v1_17(
        "file.write_text",
        r#"{"selectionReference":"AbCdEfGhIjKlMnOpQrStUv","text":"selected text"}"#,
    )))
    .expect("invalid response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let oversized = JsonValue::parse(&write_host.handle_json(&request_v1_17(
        "file.write_text",
        &format!(
            r#"{{"saveReference":"{reference}","text":"{}"}}"#,
            "x".repeat(MAX_FILE_TEXT_WRITE_BYTES + 1)
        ),
    )))
    .expect("oversized response is JSON");
    assert_eq!(
        field(field(&oversized, "error"), "code").as_string(),
        Some("file.text_too_large")
    );
    assert_eq!(
        writes
            .lock()
            .expect("write recorder lock is available")
            .as_slice(),
        ["selected text"]
    );

    let unavailable = JsonValue::parse(
        &file_write_host(
            vec![Capability::FileWriteText],
            CapturingSaveDialog,
            RecordingFileTextWrite::failing(FileTextWriteServiceError::Unavailable),
        )
        .handle_json(&request_v1_17(
            "file.write_text",
            &format!(r#"{{"saveReference":"{reference}","text":"private text"}}"#),
        )),
    )
    .expect("unavailable response is JSON");
    assert_eq!(
        field(field(&unavailable, "error"), "code").as_string(),
        Some("file.unavailable")
    );
    assert!(
        field(field(&unavailable, "error"), "message")
            .as_string()
            .is_some_and(|message| !message.contains("private"))
    );

    let unsupported = JsonValue::parse(&write_host.handle_json(&request_v1_16(
        "file.write_text",
        &format!(r#"{{"saveReference":"{reference}","text":"selected text"}}"#),
    )))
    .expect("unsupported response is JSON");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn selected_output_binary_is_canonical_bounded_and_separately_granted() {
    let reference = "ZyXwVuTsRqPoNmLkJiHgFe";
    let writer = RecordingFileBinaryWrite::accepting();
    let writes = Arc::clone(&writer.writes);
    let accepted_host = file_binary_write_host(vec![Capability::FileWriteBinary], writer);
    let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_22(
        "file.write_binary",
        &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AAEC_w"}}"#),
    )))
    .expect("binary response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "status").as_string(),
        Some("written")
    );
    assert_eq!(
        writes
            .lock()
            .expect("binary-write recorder lock is available")
            .as_slice(),
        [vec![0, 1, 2, 255]]
    );

    let denied_writer = RecordingFileBinaryWrite::accepting();
    let denied_writes = Arc::clone(&denied_writer.writes);
    let denied_discards = Arc::clone(&denied_writer.discarded);
    let denied = JsonValue::parse(
        &file_binary_write_host(vec![Capability::DialogSaveFile], denied_writer).handle_json(
            &request_v1_22(
                "file.write_binary",
                &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AB"}}"#),
            ),
        ),
    )
    .expect("denied response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );
    assert!(
        denied_writes
            .lock()
            .expect("binary-write recorder lock is available")
            .is_empty()
    );
    assert!(
        denied_discards
            .lock()
            .expect("binary-discard recorder lock is available")
            .is_empty()
    );

    let malformed_writer = RecordingFileBinaryWrite::accepting();
    let malformed_discards = Arc::clone(&malformed_writer.discarded);
    let malformed = JsonValue::parse(
        &file_binary_write_host(vec![Capability::FileWriteBinary], malformed_writer).handle_json(
            &request_v1_22(
                "file.write_binary",
                &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AB"}}"#),
            ),
        ),
    )
    .expect("malformed response is JSON");
    assert_eq!(
        field(field(&malformed, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
    assert_eq!(
        malformed_discards
            .lock()
            .expect("binary-discard recorder lock is available")
            .as_slice(),
        [SaveReference::new(reference).expect("reference is valid")]
    );

    let oversized_writer = RecordingFileBinaryWrite::accepting();
    let oversized_discards = Arc::clone(&oversized_writer.discarded);
    let oversized = JsonValue::parse(
        &file_binary_write_host(vec![Capability::FileWriteBinary], oversized_writer).handle_json(
            &request_v1_22(
                "file.write_binary",
                &format!(
                    r#"{{"saveReference":"{reference}","bytesBase64Url":"{}"}}"#,
                    "AAAA".repeat((anodrel_file_access::MAX_FILE_BINARY_WRITE_BYTES / 3) + 1)
                ),
            ),
        ),
    )
    .expect("oversized response is JSON");
    assert_eq!(
        field(field(&oversized, "error"), "code").as_string(),
        Some("file.binary_too_large")
    );
    assert_eq!(
        oversized_discards
            .lock()
            .expect("binary-discard recorder lock is available")
            .as_slice(),
        [SaveReference::new(reference).expect("reference is valid")]
    );

    let unavailable = JsonValue::parse(
        &file_binary_write_host(
            vec![Capability::FileWriteBinary],
            RecordingFileBinaryWrite::unavailable(),
        )
        .handle_json(&request_v1_22(
            "file.write_binary",
            &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AA"}}"#),
        )),
    )
    .expect("unavailable response is JSON");
    assert_eq!(
        field(field(&unavailable, "error"), "code").as_string(),
        Some("file.unavailable")
    );

    let unsupported = JsonValue::parse(&accepted_host.handle_json(&request_v1_21(
        "file.write_binary",
        &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AA"}}"#),
    )))
    .expect("unsupported response is JSON");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}
