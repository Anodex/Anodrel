use super::super::support::*;
use crate::*;

#[test]
fn menu_and_document_actions_share_ordered_revision_checked_delivery() {
    let mailbox = UiInputMailbox::new();
    let host = CoreHost::with_session_components_and_service_bundle(
        HostPolicy::new(
            "test.application",
            vec![
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::MenuWrite,
            ],
            "test-host",
        )
        .expect("test policy is valid"),
        mailbox.clone(),
        SessionCloseSignal::default(),
        HostServices::unavailable().with_menu(RecordingMenu::default()),
    );
    let document = valid_ui_document("Continue");
    let document_response = JsonValue::parse(&host.handle_json(&request_v1_1(
        "ui.document.replace",
        &ui_document_payload(&document),
    )))
    .expect("document response is JSON");
    assert_eq!(
        field(field(&document_response, "result"), "revision").as_string(),
        Some("1")
    );
    let document_revision = host
        .take_ui_document_update()
        .expect("accepted document is available")
        .revision();

    let menu_payload = r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":true}]}]}"#;
    let menu_response =
        JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", menu_payload)))
            .expect("menu response is JSON");
    assert_eq!(
        field(field(&menu_response, "result"), "revision").as_string(),
        Some("1")
    );
    let menu_revision = anodrel_menu::MenuRevision::INITIAL
        .next()
        .expect("first menu revision exists");
    let menu_action =
        anodrel_menu::MenuActionId::new("document.new").expect("test menu action is valid");

    mailbox.push(UiInputCandidate::new(
        document_revision,
        UiEvent::ActionInvoked(ElementId::new("root").expect("test ID is valid")),
    ));
    mailbox.push(MenuInputCandidate::new(menu_revision, menu_action.clone()));
    let read = JsonValue::parse(&host.handle_json(&request_v1_18("ui.events.read", "{}")))
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
    assert_eq!(events.len(), 2);
    assert_eq!(
        field(&events[0], "eventName").as_string(),
        Some("ui.action.invoked")
    );
    assert_eq!(
        field(&events[1], "eventName").as_string(),
        Some("menu.action.invoked")
    );
    assert_eq!(field(&events[1], "source").as_string(), Some("native.menu"));
    assert_eq!(
        field(field(&events[1], "schemaVersion"), "minor"),
        &JsonValue::Number("18".to_owned())
    );
    assert_eq!(
        field(field(&events[1], "payload"), "menuRevision").as_string(),
        Some("1")
    );
    assert_eq!(
        field(field(&events[1], "payload"), "action").as_string(),
        Some("document.new")
    );

    let disabled = r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":false}]}]}"#;
    let _ = host.handle_json(&request_v1_18("menu.replace", disabled));
    mailbox.push(MenuInputCandidate::new(menu_revision, menu_action));
    let stale = JsonValue::parse(&host.handle_json(&request_v1_18("ui.events.read", "{}")))
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
