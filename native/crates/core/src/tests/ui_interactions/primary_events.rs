use super::super::support::*;
use crate::*;

#[test]
fn reads_only_current_enabled_ui_actions_from_the_supplied_input_mailbox() {
    let mailbox = UiInputMailbox::new();
    let host = CoreHost::with_ui_input_mailbox(
        HostPolicy::new(
            "test.application",
            vec![Capability::UiDocumentWrite, Capability::UiEventsRead],
            "test-host",
        )
        .expect("test policy is valid"),
        mailbox.clone(),
    );
    let document = valid_ui_document("Continue");
    let update = JsonValue::parse(&host.handle_json(&request_v1_1(
        "ui.document.replace",
        &ui_document_payload(&document),
    )))
    .expect("update response is JSON");
    assert_eq!(
        field(field(&update, "result"), "revision").as_string(),
        Some("1")
    );

    let current = host
        .take_ui_document_update()
        .expect("accepted document is available")
        .revision();
    let action = UiEvent::ActionInvoked(ElementId::new("root").expect("test ID is valid"));
    mailbox.push(UiInputCandidate::new(current, action.clone()));
    let read = JsonValue::parse(&host.handle_json(&request_v1_2("ui.events.read", "{}")))
        .expect("event response is JSON");
    let result = field(&read, "result");
    assert_eq!(field(result, "dropped"), &JsonValue::Number("0".to_owned()));
    assert_eq!(
        field(result, "discarded"),
        &JsonValue::Number("0".to_owned())
    );
    let JsonValue::Array(events) = field(result, "events") else {
        panic!("events is an array");
    };
    assert_eq!(events.len(), 1);
    assert_eq!(
        field(&events[0], "eventName").as_string(),
        Some("ui.action.invoked")
    );
    assert_eq!(
        field(field(&events[0], "payload"), "action").as_string(),
        Some("root")
    );

    let replacement = valid_ui_document("Continue safely");
    let _ = host.handle_json(&request_v1_1(
        "ui.document.replace",
        &ui_document_payload(&replacement),
    ));
    mailbox.push(UiInputCandidate::new(current, action));
    let stale = JsonValue::parse(&host.handle_json(&request_v1_2("ui.events.read", "{}")))
        .expect("stale event response is JSON");
    let JsonValue::Array(events) = field(field(&stale, "result"), "events") else {
        panic!("events is an array");
    };
    assert!(events.is_empty());
    assert_eq!(
        field(field(&stale, "result"), "discarded"),
        &JsonValue::Number("1".to_owned())
    );
}
