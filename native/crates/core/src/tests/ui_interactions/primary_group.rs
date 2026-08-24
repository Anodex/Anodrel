use super::super::support::*;
use crate::*;

#[test]
fn grouped_primary_operations_reuse_the_primary_mailboxes_and_leave_secondary_input_local() {
    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
        document_mailbox.clone(),
        input_mailbox.clone(),
    );
    let host = CoreHost::with_session_window_group_and_service_bundle(
        HostPolicy::new(
            "test.application",
            vec![Capability::UiDocumentWrite, Capability::UiEventsRead],
            "test-host",
        )
        .expect("test policy is valid"),
        group.clone(),
        SessionCloseSignal::default(),
        HostServices::unavailable(),
    );
    let document = valid_ui_document("Continue");

    let replacement = JsonValue::parse(&host.handle_json(&request_v1_1(
        "ui.document.replace",
        &ui_document_payload(&document),
    )))
    .expect("replacement response is JSON");
    assert_eq!(field(&replacement, "status").as_string(), Some("success"));
    assert!(
        host.take_ui_document_update().is_none(),
        "the group publishes directly to its primary mailbox"
    );
    let primary_snapshot = document_mailbox
        .take()
        .expect("the supplied primary mailbox receives the snapshot");
    assert_eq!(primary_snapshot.revision().value(), 1);

    let opening_group = group.clone();
    let opening_document = document.clone();
    let opening = thread::spawn(move || {
        opening_group.open_secondary(
            WindowTitleProposal::new("Secondary").expect("test title is valid"),
            &opening_document,
        )
    });
    let request = loop {
        if let Some(request) = group.take_open_request() {
            break request;
        }
        thread::yield_now();
    };
    assert!(group.complete_open(request.id(), true));
    let secondary = opening
        .join()
        .expect("opening worker does not panic")
        .expect("secondary opens");
    let secondary_resources = group
        .resources(&secondary)
        .expect("secondary resources are registered");
    secondary_resources
        .input_mailbox()
        .push(UiInputCandidate::new(
            request.snapshot().snapshot().revision(),
            UiEvent::ActionInvoked(ElementId::new("root").expect("test ID is valid")),
        ));

    let primary_read = JsonValue::parse(&host.handle_json(&request_v1_2("ui.events.read", "{}")))
        .expect("event response is JSON");
    let JsonValue::Array(events) = field(field(&primary_read, "result"), "events") else {
        panic!("events is an array");
    };
    assert!(events.is_empty());
    assert_eq!(
        group
            .drain_input_batch(&secondary)
            .expect("secondary remains registered")
            .into_candidates()
            .len(),
        1,
        "targetless primary reads cannot consume a secondary view's input"
    );
}
