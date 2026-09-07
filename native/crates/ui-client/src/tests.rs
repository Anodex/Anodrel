use anodrel_json::JsonValue;

use crate::{
    MAX_ACTIONS_PER_BATCH, MAX_WINDOW_ACTIONS_PER_BATCH, SessionWindowId, UiActionBatch,
    UiClientError, UiEvent,
};

mod file_write;
mod foundation;
mod notification;
mod support;
mod tray;
mod validation;
mod window_controls;

use support::{
    CONTEXT_MENU, DOCUMENT, MENU, SCROLL_DOCUMENT, STATUS_DOCUMENT, context_menu_event_batch,
    event_batch, field_snapshot, menu_event_batch, messages, request_field, request_protocol_minor,
    response, session_with_responses, window_event_batch, window_event_batch_with_count,
    window_event_batch_with_count_per_window,
};

#[test]
fn field_snapshots_use_one_exact_protocol_1_15_whole_surface_read() {
    let (mut session, written) = session_with_responses([response(
        "anodrel-ui-1",
        &field_snapshot(&[("template.form.name", "Ada")]),
    )]);

    let snapshot = session.read_fields().expect("field snapshot is typed");
    let [field] = snapshot.fields() else {
        panic!("the fixed response has one field");
    };
    assert_eq!(field.id().as_str(), "template.form.name");
    assert_eq!(field.value(), "Ada");

    let messages = messages(&written);
    assert_eq!(
        request_field(&messages[1], "operation"),
        Some("ui.fields.read".to_owned())
    );
    assert_eq!(request_protocol_minor(&messages[1]), Some(15));
    let request = JsonValue::parse(&messages[1]).expect("field read request is JSON");
    assert_eq!(
        request.as_object().and_then(|fields| fields.get("payload")),
        Some(&JsonValue::Object(Default::default()))
    );
}

#[test]
fn malformed_or_out_of_order_field_snapshots_fail_closed() {
    for result in [
        r#"{"fields":[{"id":"template.form.name","value":"Ada"},{"id":"template.form.name","value":"Grace"}]}"#,
        r#"{"fields":[{"id":"zeta","value":"Ada"},{"id":"alpha","value":"Grace"}]}"#,
        r#"{"fields":[{"id":"template.form.name","value":"one\ntwo"}]}"#,
        r#"{"fields":[{"id":"template.form.name","value":"Ada","edited":true}]}"#,
        r#"{"fields":[],"focus":"template.form.name"}"#,
    ] {
        let (mut session, _) = session_with_responses([response("anodrel-ui-1", result)]);
        assert_eq!(
            session.read_fields(),
            Err(UiClientError::ResponseInvalid),
            "invalid field response must not become a typed snapshot: {result}"
        );
    }
}

#[test]
fn native_menu_session_keeps_v1_24_replacement_and_uses_the_current_event_reader() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"revision":"1"}"#),
        response("anodrel-ui-2", r#"{"revision":"1"}"#),
        response(
            "anodrel-ui-3",
            &menu_event_batch("template.menu.complete", "1"),
        ),
        response("anodrel-ui-4", r#"{"status":"accepted"}"#),
    ]);

    assert_eq!(
        session
            .replace_document_v1(DOCUMENT)
            .expect("document is accepted")
            .value(),
        1
    );
    assert_eq!(
        session
            .replace_menu_v1(MENU)
            .expect("menu is accepted")
            .value(),
        1
    );
    let batch = session.read_events().expect("menu event batch is typed");
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.discarded(), 0);
    let [UiEvent::MenuAction(action)] = batch.events() else {
        panic!("the fixed native menu event is preserved");
    };
    assert_eq!(action.revision().value(), 1);
    assert_eq!(action.action(), "template.menu.complete");
    session.close().expect("close is accepted");

    let messages = messages(&written);
    let minors = messages
        .iter()
        .skip(1)
        .map(|message| request_protocol_minor(message))
        .collect::<Vec<_>>();
    assert_eq!(minors, [Some(3), Some(24), Some(33), Some(3)]);
    let menu_request = JsonValue::parse(&messages[2]).expect("menu request is JSON");
    assert_eq!(
        menu_request
            .as_object()
            .and_then(|fields| fields.get("payload")),
        Some(&JsonValue::parse(MENU).expect("fixed menu is JSON"))
    );
}

#[test]
fn native_context_menu_session_uses_the_fixed_protocol_1_32_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"revision":"1"}"#),
        response(
            "anodrel-ui-2",
            &context_menu_event_batch("template.context.complete", "1"),
        ),
        response("anodrel-ui-3", r#"{"status":"accepted"}"#),
    ]);

    assert_eq!(
        session
            .replace_context_menu_v1(CONTEXT_MENU)
            .expect("context menu is accepted")
            .value(),
        1
    );
    let batch = session
        .read_context_menu_actions()
        .expect("context-menu action batch is typed");
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.discarded(), 0);
    let [action] = batch.actions() else {
        panic!("the fixed native context-menu action is preserved");
    };
    assert_eq!(action.revision().value(), 1);
    assert_eq!(action.action(), "template.context.complete");
    session.close().expect("close is accepted");

    let messages = messages(&written);
    let operations = messages
        .iter()
        .skip(1)
        .map(|message| request_field(message, "operation"))
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        [
            Some("menu.context.replace".to_owned()),
            Some("ui.events.read".to_owned()),
            Some("session.close".to_owned()),
        ]
    );
    let minors = messages
        .iter()
        .skip(1)
        .map(|message| request_protocol_minor(message))
        .collect::<Vec<_>>();
    assert_eq!(minors, [Some(32), Some(32), Some(3)]);
    let context_request = JsonValue::parse(&messages[1]).expect("context-menu request is JSON");
    assert_eq!(
        context_request
            .as_object()
            .and_then(|fields| fields.get("payload")),
        Some(&JsonValue::parse(CONTEXT_MENU).expect("fixed context menu is JSON"))
    );
}

#[test]
fn multi_window_session_uses_only_the_fixed_protocol_1_25_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"windowId":"window-1"}"#),
        response("anodrel-ui-2", r#"{"revision":"2"}"#),
        response(
            "anodrel-ui-3",
            &window_event_batch("window-1", "template.window.complete", "2"),
        ),
        response("anodrel-ui-4", r#"{"status":"requested"}"#),
    ]);

    let secondary = session
        .open_window_v1("Template secondary", DOCUMENT)
        .expect("secondary view is accepted");
    assert_eq!(secondary.to_string(), "window-1");
    assert_eq!(
        session
            .replace_window_document_v1(secondary, DOCUMENT)
            .expect("secondary replacement is accepted")
            .value(),
        2
    );
    let batch = session
        .read_window_actions()
        .expect("tagged action batch is typed");
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.discarded(), 0);
    let [action] = batch.actions() else {
        panic!("the one tagged action is preserved");
    };
    assert_eq!(action.window(), SessionWindowId::Secondary(secondary));
    assert_eq!(action.action().revision().value(), 2);
    assert_eq!(action.action().action(), "template.window.complete");
    session
        .close_window(secondary)
        .expect("secondary close is accepted");

    let messages = messages(&written);
    let operations = messages
        .iter()
        .skip(1)
        .map(|message| request_field(message, "operation"))
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        [
            Some("window.open".to_owned()),
            Some("ui.document.replace.window".to_owned()),
            Some("ui.events.read.window".to_owned()),
            Some("window.close".to_owned()),
        ]
    );
    assert!(
        messages
            .iter()
            .skip(1)
            .all(|message| { request_protocol_minor(message) == Some(25) })
    );
    let open = JsonValue::parse(&messages[1]).expect("open request is JSON");
    let open_payload = open
        .as_object()
        .and_then(|fields| fields.get("payload"))
        .expect("open has payload");
    assert_eq!(
        open_payload,
        &JsonValue::parse(&format!(
            r#"{{"title":"Template secondary","document":{}}}"#,
            JsonValue::String(DOCUMENT.to_owned()).to_json()
        ))
        .expect("expected open payload is JSON")
    );
}

#[test]
fn live_status_documents_use_only_the_explicit_protocol_1_26_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"revision":"1"}"#),
        response("anodrel-ui-2", r#"{"windowId":"window-1"}"#),
        response("anodrel-ui-3", r#"{"revision":"2"}"#),
    ]);

    assert_eq!(
        session
            .replace_document_v3(STATUS_DOCUMENT)
            .expect("primary status update is accepted")
            .value(),
        1
    );
    let secondary = session
        .open_window_v3("Status", STATUS_DOCUMENT)
        .expect("status secondary is accepted");
    assert_eq!(
        session
            .replace_window_document_v3(secondary, STATUS_DOCUMENT)
            .expect("status secondary update is accepted")
            .value(),
        2
    );

    let messages = messages(&written);
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_field(message, "operation"))
            .collect::<Vec<_>>(),
        [
            Some("ui.document.replace.v3".to_owned()),
            Some("window.open.v3".to_owned()),
            Some("ui.document.replace.window.v3".to_owned()),
        ]
    );
    assert!(
        messages
            .iter()
            .skip(1)
            .all(|message| request_protocol_minor(message) == Some(26))
    );
}

#[test]
fn secondary_scroll_documents_use_only_the_explicit_protocol_1_27_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"windowId":"window-1"}"#),
        response("anodrel-ui-2", r#"{"revision":"2"}"#),
    ]);

    let secondary = session
        .open_window_v2("Scrollable notes", SCROLL_DOCUMENT)
        .expect("scroll secondary is accepted");
    assert_eq!(
        session
            .replace_window_document_v2(secondary, SCROLL_DOCUMENT)
            .expect("scroll secondary update is accepted")
            .value(),
        2
    );

    let messages = messages(&written);
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_field(message, "operation"))
            .collect::<Vec<_>>(),
        [
            Some("window.open.v2".to_owned()),
            Some("ui.document.replace.window.v2".to_owned()),
        ]
    );
    assert!(
        messages
            .iter()
            .skip(1)
            .all(|message| request_protocol_minor(message) == Some(27))
    );
}

#[test]
fn multi_window_action_reads_accept_one_full_group_batch() {
    let (mut session, _) = session_with_responses([response(
        "anodrel-ui-1",
        &window_event_batch_with_count_per_window(MAX_ACTIONS_PER_BATCH),
    )]);

    let batch = session
        .read_window_actions()
        .expect("a full bounded group batch is typed");
    assert_eq!(batch.actions().len(), MAX_WINDOW_ACTIONS_PER_BATCH);
    let mut per_window = std::collections::BTreeMap::new();
    for action in batch.actions() {
        let identity = match action.window() {
            SessionWindowId::Main => "main".to_owned(),
            SessionWindowId::Secondary(window) => window.to_string(),
        };
        *per_window.entry(identity).or_insert(0_usize) += 1;
    }
    assert_eq!(per_window.len(), 4);
    assert!(
        per_window
            .values()
            .all(|count| *count == MAX_ACTIONS_PER_BATCH)
    );
}

#[test]
fn multi_window_action_reads_reject_more_than_one_view_queue_can_hold() {
    let (mut session, _) = session_with_responses([response(
        "anodrel-ui-1",
        &window_event_batch_with_count("main", MAX_ACTIONS_PER_BATCH + 1),
    )]);

    assert_eq!(
        session.read_window_actions(),
        Err(UiClientError::ResponseInvalid)
    );
}
