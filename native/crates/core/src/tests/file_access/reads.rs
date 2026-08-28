//! Retained open-file selection read capability tests.

use super::super::support::*;
use crate::*;

#[test]
fn selected_file_text_is_separately_granted_bounded_and_safe() {
    let reference = "AbCdEfGhIjKlMnOpQrStUv";
    let accepted_host = file_access_host(
        vec![Capability::FileReadText],
        CapturingFileDialog,
        FixedFileText(Ok("selected text".to_owned())),
    );
    let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
        "file.read_text",
        &format!(r#"{{"selectionReference":"{reference}"}}"#),
    )))
    .expect("text response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "text").as_string(),
        Some("selected text")
    );

    let denied = JsonValue::parse(&host(vec![Capability::DialogOpenFile]).handle_json(
        &request_v1_9(
            "file.read_text",
            &format!(r#"{{"selectionReference":"{reference}"}}"#),
        ),
    ))
    .expect("denied response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
        "file.read_text",
        r#"{"selectionReference":"path.txt"}"#,
    )))
    .expect("invalid response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    for (service_error, expected) in [
        (FileTextServiceError::Unavailable, "file.unavailable"),
        (FileTextServiceError::InvalidText, "file.text_invalid"),
        (FileTextServiceError::TooLarge, "file.text_too_large"),
    ] {
        let failing_host = file_access_host(
            vec![Capability::FileReadText],
            CapturingFileDialog,
            FixedFileText(Err(service_error)),
        );
        let response = JsonValue::parse(&failing_host.handle_json(&request_v1_9(
            "file.read_text",
            &format!(r#"{{"selectionReference":"{reference}"}}"#),
        )))
        .expect("failure response is JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some(expected)
        );
    }

    let oversized_host = file_access_host(
        vec![Capability::FileReadText],
        CapturingFileDialog,
        FixedFileText(Ok("x".repeat(MAX_FILE_TEXT_RESPONSE_BYTES + 1))),
    );
    let oversized = JsonValue::parse(&oversized_host.handle_json(&request_v1_9(
        "file.read_text",
        &format!(r#"{{"selectionReference":"{reference}"}}"#),
    )))
    .expect("oversized response is JSON");
    assert_eq!(
        field(field(&oversized, "error"), "code").as_string(),
        Some("file.text_too_large")
    );
}
