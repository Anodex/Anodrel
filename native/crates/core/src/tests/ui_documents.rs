use super::support::*;
use crate::*;

#[test]
fn replaces_ui_documents_only_with_the_current_capability_and_protocol_minor() {
    let document = valid_ui_document("Continue");
    let update_request = request_v1_1("ui.document.replace", &ui_document_payload(&document));

    let denied = JsonValue::parse(&host(vec![]).handle_json(&update_request))
        .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let host = host(vec![Capability::UiDocumentWrite]);
    let first =
        JsonValue::parse(&host.handle_json(&update_request)).expect("response JSON is valid");
    assert_eq!(field(&first, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&first, "result"), "revision").as_string(),
        Some("1")
    );
    let first_snapshot = host
        .take_ui_document_update()
        .expect("accepted document is available to the transport");
    assert_eq!(first_snapshot.revision().value(), 1);
    assert_eq!(first_snapshot.document().root().id().as_str(), "root");
    assert!(host.take_ui_document_update().is_none());

    let invalid = request_v1_1("ui.document.replace", &ui_document_payload("not JSON"));
    let invalid = JsonValue::parse(&host.handle_json(&invalid)).expect("response JSON is valid");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let second_document = valid_ui_document("Continue safely");
    let second = JsonValue::parse(&host.handle_json(&request_v1_1(
        "ui.document.replace",
        &ui_document_payload(&second_document),
    )))
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&second, "result"), "revision").as_string(),
        Some("2")
    );

    let old_minor = JsonValue::parse(&host.handle_json(&request(
        "ui.document.replace",
        &ui_document_payload(&document),
    )))
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&old_minor, "error"), "code").as_string(),
        Some("operation.unsupported")
    );

    let oversized = request_v1_1(
        "ui.document.replace",
        &ui_document_payload(&"x".repeat(MAX_UI_DOCUMENT_REQUEST_BYTES + 1)),
    );
    let oversized =
        JsonValue::parse(&host.handle_json(&oversized)).expect("response JSON is valid");
    assert_eq!(
        field(field(&oversized, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
}

#[test]
fn replaces_version_two_documents_only_through_the_new_operation() {
    let host = host(vec![Capability::UiDocumentWrite]);
    let document = valid_ui_document_v2();

    let accepted = JsonValue::parse(&host.handle_json(&request_v1_4(
        "ui.document.replace.v2",
        &ui_document_payload(document),
    )))
    .expect("response JSON is valid");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    let snapshot = host
        .take_ui_document_update()
        .expect("accepted version two document is delivered");
    assert_eq!(snapshot.document().root().id().as_str(), "viewport");

    let wrong_operation = JsonValue::parse(&host.handle_json(&request_v1_1(
        "ui.document.replace",
        &ui_document_payload(document),
    )))
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&wrong_operation, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
}

#[test]
fn replaces_version_three_status_documents_only_through_protocol_1_26() {
    let host = host(vec![Capability::UiDocumentWrite]);
    let document = valid_ui_document_v3("Saved", "polite");

    let accepted = JsonValue::parse(&host.handle_json(&request_v1_26(
        "ui.document.replace.v3",
        &ui_document_payload(&document),
    )))
    .expect("response JSON is valid");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    let snapshot = host
        .take_ui_document_update()
        .expect("accepted version three document is delivered");
    assert_eq!(
        snapshot.document().status().map(|status| status.value()),
        Some("Saved")
    );

    let wrong_operation = JsonValue::parse(&host.handle_json(&request_v1_4(
        "ui.document.replace.v2",
        &ui_document_payload(&document),
    )))
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&wrong_operation, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let old_minor = JsonValue::parse(&host.handle_json(&request_v1_25(
        "ui.document.replace.v3",
        &ui_document_payload(&document),
    )))
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&old_minor, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}
