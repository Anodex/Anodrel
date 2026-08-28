use super::super::support::*;
use crate::*;
use anodrel_ui_session::UiWindowGroup;

#[test]
fn protocol_v1_25_opens_targets_reads_and_closes_only_session_owned_views() {
    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
        document_mailbox,
        input_mailbox,
    );
    let host = CoreHost::with_session_window_group_and_service_bundle(
        HostPolicy::new(
            "test.application",
            vec![
                Capability::WindowOpen,
                Capability::WindowClose,
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
            ],
            "test-host",
        )
        .expect("test policy is valid"),
        group.clone(),
        SessionCloseSignal::default(),
        HostServices::unavailable(),
    );
    let document = valid_ui_document("Secondary action");
    let opening_group = group.clone();
    let native_creator = thread::spawn(move || {
        loop {
            if let Some(request) = opening_group.take_open_request() {
                assert_eq!(request.context().as_str(), "Notes");
                assert!(opening_group.complete_open(request.id(), true));
                break;
            }
            thread::yield_now();
        }
    });

    let open_payload = object([
        ("document", JsonValue::String(document.clone())),
        ("title", JsonValue::String("Notes".to_owned())),
    ])
    .to_json();
    let opened = JsonValue::parse(&host.handle_json(&request_v1_25("window.open", &open_payload)))
        .expect("open response is JSON");
    native_creator
        .join()
        .expect("native group creator does not panic");
    assert_eq!(field(&opened, "status").as_string(), Some("success"));
    let window_id = field(field(&opened, "result"), "windowId")
        .as_string()
        .expect("open result carries an identity");
    assert_eq!(window_id, "window-1");
    let secondary = UiWindowId::parse(window_id).expect("fixed secondary ID parses");

    let replacement_payload = object([
        ("document", JsonValue::String(document.clone())),
        ("windowId", JsonValue::String(window_id.to_owned())),
    ])
    .to_json();
    let replacement = JsonValue::parse(&host.handle_json(&request_v1_25(
        "ui.document.replace.window",
        &replacement_payload,
    )))
    .expect("replacement response is JSON");
    assert_eq!(
        field(field(&replacement, "result"), "revision").as_string(),
        Some("2")
    );

    let secondary_resources = group
        .resources(&secondary)
        .expect("secondary resources remain available");
    let revision = secondary_resources
        .document_mailbox()
        .take()
        .expect("targeted replacement publishes the secondary snapshot")
        .revision();
    secondary_resources
        .input_mailbox()
        .push(UiInputCandidate::new(
            revision,
            UiEvent::ActionInvoked(ElementId::new("root").expect("fixed action ID is valid")),
        ));
    let events = JsonValue::parse(&host.handle_json(&request_v1_25("ui.events.read.window", "{}")))
        .expect("events response is JSON");
    let JsonValue::Array(events) = field(field(&events, "result"), "events") else {
        panic!("events result is an array");
    };
    assert_eq!(events.len(), 1);
    assert_eq!(field(&events[0], "windowId").as_string(), Some("window-1"));
    assert_eq!(
        field(&events[0], "eventName").as_string(),
        Some("ui.action.invoked")
    );

    let close_payload = object([("windowId", JsonValue::String(window_id.to_owned()))]).to_json();
    let close = JsonValue::parse(&host.handle_json(&request_v1_25("window.close", &close_payload)))
        .expect("close response is JSON");
    assert_eq!(
        field(field(&close, "result"), "status").as_string(),
        Some("requested")
    );
    assert_eq!(
        group.take_secondary_close_requests(),
        vec![secondary.clone()]
    );
    assert!(group.close_secondary(&secondary).is_ok());

    let unavailable = JsonValue::parse(&host.handle_json(&request_v1_25(
        "ui.document.replace.window",
        &replacement_payload,
    )))
    .expect("unavailable response is JSON");
    assert_eq!(
        field(field(&unavailable, "error"), "code").as_string(),
        Some("window.unavailable")
    );
}
