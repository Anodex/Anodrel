use super::super::support::*;
use crate::*;
use anodrel_ui_session::UiWindowGroup;

#[test]
fn protocol_v1_27_keeps_scroll_documents_explicit_for_secondary_views() {
    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
        document_mailbox,
        input_mailbox,
    );
    let host = CoreHost::with_session_window_group_and_service_bundle(
        HostPolicy::new(
            "test.application",
            vec![Capability::WindowOpen, Capability::UiDocumentWrite],
            "test-host",
        )
        .expect("test policy is valid"),
        group.clone(),
        SessionCloseSignal::default(),
        HostServices::unavailable(),
    );
    let initial = valid_ui_document_v2();
    let opening_group = group.clone();
    let native_creator = thread::spawn(move || {
        loop {
            if let Some(request) = opening_group.take_open_request() {
                assert!(opening_group.complete_open(request.id(), true));
                break;
            }
            thread::yield_now();
        }
    });

    let opened = JsonValue::parse(
        &host.handle_json(&request_v1_27(
            "window.open.v2",
            &object([
                ("document", JsonValue::String(initial.to_owned())),
                ("title", JsonValue::String("Scrollable notes".to_owned())),
            ])
            .to_json(),
        )),
    )
    .expect("open response is JSON");
    native_creator
        .join()
        .expect("native group creator does not panic");
    let window_id = field(field(&opened, "result"), "windowId")
        .as_string()
        .expect("open result carries an identity");
    let secondary = UiWindowId::parse(window_id).expect("fixed secondary ID parses");
    let resources = group
        .resources(&secondary)
        .expect("secondary is registered");
    let initial_snapshot = resources
        .document_mailbox()
        .take()
        .expect("initial v2 snapshot is published");
    assert_eq!(initial_snapshot.revision().value(), 1);
    assert_eq!(initial_snapshot.document().root().id().as_str(), "viewport");

    let replacement = JsonValue::parse(
        &host.handle_json(&request_v1_27(
            "ui.document.replace.window.v2",
            &object([
                ("document", JsonValue::String(initial.to_owned())),
                ("windowId", JsonValue::String(window_id.to_owned())),
            ])
            .to_json(),
        )),
    )
    .expect("replacement response is JSON");
    assert_eq!(
        field(field(&replacement, "result"), "revision").as_string(),
        Some("2")
    );
    assert_eq!(
        resources
            .document_mailbox()
            .take()
            .expect("updated v2 snapshot is published")
            .revision()
            .value(),
        2
    );

    let old_minor = JsonValue::parse(
        &host.handle_json(&request_v1_26(
            "window.open.v2",
            &object([
                ("document", JsonValue::String(initial.to_owned())),
                ("title", JsonValue::String("Scrollable notes".to_owned())),
            ])
            .to_json(),
        )),
    )
    .expect("old-version response is JSON");
    assert_eq!(
        field(field(&old_minor, "error"), "code").as_string(),
        Some("operation.unsupported")
    );

    let legacy = JsonValue::parse(
        &host.handle_json(&request_v1_27(
            "window.open",
            &object([
                ("document", JsonValue::String(initial.to_owned())),
                ("title", JsonValue::String("Scrollable notes".to_owned())),
            ])
            .to_json(),
        )),
    )
    .expect("legacy response is JSON");
    assert_eq!(
        field(field(&legacy, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
}
