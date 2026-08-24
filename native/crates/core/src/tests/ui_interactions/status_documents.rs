use super::super::support::*;
use crate::*;

#[test]
fn protocol_v1_26_keeps_status_documents_explicit_for_secondary_views() {
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
    let initial = valid_ui_document_v3("Saved", "polite");
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
        &host.handle_json(&request_v1_26(
            "window.open.v3",
            &object([
                ("document", JsonValue::String(initial.clone())),
                ("title", JsonValue::String("Status".to_owned())),
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
        .expect("initial v3 snapshot is published");
    assert_eq!(
        initial_snapshot
            .document()
            .status()
            .map(|status| status.value()),
        Some("Saved")
    );

    let updated = valid_ui_document_v3("Save failed", "assertive");
    let replacement = JsonValue::parse(
        &host.handle_json(&request_v1_26(
            "ui.document.replace.window.v3",
            &object([
                ("document", JsonValue::String(updated)),
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
    let replacement_snapshot = resources
        .document_mailbox()
        .take()
        .expect("updated v3 snapshot is published");
    assert_eq!(
        replacement_snapshot
            .document()
            .status()
            .map(|status| status.value()),
        Some("Save failed")
    );

    let v1_refusal = JsonValue::parse(
        &host.handle_json(&request_v1_25(
            "window.open.v3",
            &object([
                ("document", JsonValue::String(initial)),
                ("title", JsonValue::String("Status".to_owned())),
            ])
            .to_json(),
        )),
    )
    .expect("old-version response is JSON");
    assert_eq!(
        field(field(&v1_refusal, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}
