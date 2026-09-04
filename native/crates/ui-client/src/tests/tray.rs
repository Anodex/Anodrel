//! Typed tray-session protocol coverage.

use super::{
    JsonValue, UiEvent, messages, request_field, request_protocol_minor, response,
    session_with_responses,
};

const TRAY: &str =
    r#"{"items":[{"id":"template.tray.open","label":"Open window","enabled":true}]}"#;

#[test]
fn native_tray_session_uses_the_fixed_protocol_1_33_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"revision":"1"}"#),
        response("anodrel-ui-2", &tray_event_batch("template.tray.open", "1")),
        response("anodrel-ui-3", r#"{"status":"accepted"}"#),
    ]);

    assert_eq!(
        session
            .replace_tray_v1(TRAY)
            .expect("tray is accepted")
            .value(),
        1
    );
    let batch = session
        .read_tray_actions()
        .expect("tray action batch is typed");
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.discarded(), 0);
    let [action] = batch.actions() else {
        panic!("the fixed native tray action is preserved");
    };
    assert_eq!(action.revision().value(), 1);
    assert_eq!(action.action(), "template.tray.open");
    session.close().expect("close is accepted");

    let messages = messages(&written);
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_field(message, "operation"))
            .collect::<Vec<_>>(),
        [
            Some("tray.replace".to_owned()),
            Some("ui.events.read".to_owned()),
            Some("session.close".to_owned()),
        ]
    );
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_protocol_minor(message))
            .collect::<Vec<_>>(),
        [Some(33), Some(33), Some(3)]
    );
    let tray_request = JsonValue::parse(&messages[1]).expect("tray request is JSON");
    assert_eq!(
        tray_request
            .as_object()
            .and_then(|fields| fields.get("payload")),
        Some(&JsonValue::parse(TRAY).expect("fixed tray is JSON"))
    );
}

#[test]
fn general_event_reads_accept_the_typed_tray_variant_only_at_protocol_1_33() {
    let (mut session, written) = session_with_responses([response(
        "anodrel-ui-1",
        &tray_event_batch("template.tray.open", "1"),
    )]);

    let batch = session.read_events().expect("generic event batch is typed");
    let [UiEvent::TrayAction(action)] = batch.events() else {
        panic!("the native tray event is preserved");
    };
    assert_eq!(action.revision().value(), 1);
    assert_eq!(action.action(), "template.tray.open");

    let messages = messages(&written);
    assert_eq!(request_protocol_minor(&messages[1]), Some(33));
}

fn tray_event_batch(action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"tray.action.invoked","source":"native.tray","protocolVersion":{{"major":1,"minor":33}},"schemaVersion":{{"major":1,"minor":33}},"payload":{{"trayRevision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}
