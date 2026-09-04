//! Failure-boundary tests for the typed UI client.

use super::*;

#[test]
fn document_only_event_reads_fail_closed_when_a_menu_event_arrives() {
    let (mut session, _) = session_with_responses([response(
        "anodrel-ui-1",
        &menu_event_batch("template.menu.complete", "1"),
    )]);

    assert_eq!(session.read_actions(), Err(UiClientError::ResponseInvalid));
}

#[test]
fn unexpected_or_invalid_events_never_become_typed_actions() {
    let malformed = r#"{"events":[{"kind":"event","eventName":"ui.action.invoked","source":"native.ui","protocolVersion":{"major":1,"minor":2},"schemaVersion":{"major":1,"minor":0},"payload":{"revision":"1","action":"not valid"}}],"dropped":0,"discarded":0}"#;
    let (mut session, _) = session_with_responses([response("anodrel-ui-1", malformed)]);

    assert_eq!(session.read_actions(), Err(UiClientError::ResponseInvalid));
}

#[test]
fn invalid_documents_fail_before_they_can_create_a_protocol_request() {
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.replace_document_v1("not a document"),
        Err(UiClientError::DocumentInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn invalid_menus_fail_before_they_can_create_a_protocol_request() {
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.replace_menu_v1(r#"{"menus":[]}"#),
        Err(UiClientError::MenuInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn invalid_context_menus_fail_before_they_can_create_a_protocol_request() {
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.replace_context_menu_v1(r#"{"items":[]}"#),
        Err(UiClientError::ContextMenuInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn invalid_tray_menus_fail_before_they_can_create_a_protocol_request() {
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.replace_tray_v1(r#"{"items":[]}"#),
        Err(UiClientError::TrayInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn invalid_multi_window_title_or_identity_fails_closed() {
    let (mut session, written) = session_with_responses([]);
    assert_eq!(
        session.open_window_v1("unsafe\nwindow title", DOCUMENT),
        Err(UiClientError::WindowTitleInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "an invalid title cannot create a request"
    );

    let (mut session, _) =
        session_with_responses([response("anodrel-ui-1", r#"{"windowId":"main"}"#)]);
    assert_eq!(
        session.open_window_v1("Ordinary", DOCUMENT),
        Err(UiClientError::ResponseInvalid),
        "a host must never claim that primary is an opened secondary"
    );
}

#[test]
fn menu_revisions_must_be_nonzero_canonical_decimal_values() {
    let (mut session, _) =
        session_with_responses([response("anodrel-ui-1", r#"{"revision":"01"}"#)]);

    assert_eq!(
        session.replace_menu_v1(MENU),
        Err(UiClientError::ResponseInvalid)
    );
}

#[test]
fn context_menu_events_require_their_own_schema_and_revision_fields() {
    let malformed = r#"{"events":[{"kind":"event","eventName":"menu.context.action.invoked","source":"native.context_menu","protocolVersion":{"major":1,"minor":32},"schemaVersion":{"major":1,"minor":32},"payload":{"menuRevision":"1","action":"template.context.complete"}}],"dropped":0,"discarded":0}"#;
    let (mut session, _) = session_with_responses([response("anodrel-ui-1", malformed)]);

    assert_eq!(
        session.read_context_menu_actions(),
        Err(UiClientError::ResponseInvalid)
    );
}

#[test]
fn tray_events_require_their_own_schema_and_revision_fields() {
    let malformed = r#"{"events":[{"kind":"event","eventName":"tray.action.invoked","source":"native.tray","protocolVersion":{"major":1,"minor":33},"schemaVersion":{"major":1,"minor":33},"payload":{"contextMenuRevision":"1","action":"template.tray.open"}}],"dropped":0,"discarded":0}"#;
    let (mut session, _) = session_with_responses([response("anodrel-ui-1", malformed)]);

    assert_eq!(
        session.read_tray_actions(),
        Err(UiClientError::ResponseInvalid)
    );
}

#[test]
fn documents_over_the_operation_limit_fail_before_they_can_create_a_request() {
    let too_large_but_otherwise_valid = format!(
        r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"root","kind":"text","value":"{}","fontSize":16,"tone":"primary"}}}}"#,
        "x".repeat(24 * 1024)
    );
    assert!(
        anodrel_ui_document::decode(&too_large_but_otherwise_valid).is_ok(),
        "the operation limit, not the document codec, must reject this fixture"
    );
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.replace_document_v1(&too_large_but_otherwise_valid),
        Err(UiClientError::DocumentInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn batches_refuse_noncanonical_counts_and_revisions() {
    let invalid_count =
        JsonValue::parse(r#"{"events":[],"dropped":-1,"discarded":0}"#).expect("fixture is JSON");
    assert_eq!(
        UiActionBatch::parse(&invalid_count),
        Err(UiClientError::ResponseInvalid)
    );
    let invalid_revision =
        JsonValue::parse(&event_batch("template.complete", "01")).expect("fixture is JSON");
    assert_eq!(
        UiActionBatch::parse(&invalid_revision),
        Err(UiClientError::ResponseInvalid)
    );
}
