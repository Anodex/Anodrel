use super::support::*;
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

#[test]
fn accepts_only_a_granted_current_protocol_session_close_request() {
    let signal = SessionCloseSignal::default();
    let close_host = CoreHost::with_session_components(
        HostPolicy::new(
            "test.application",
            vec![Capability::SessionClose],
            "test-host",
        )
        .expect("test policy is valid"),
        UiInputMailbox::new(),
        signal.clone(),
    );
    let accepted = JsonValue::parse(&close_host.handle_json(&request_v1_3("session.close", "{}")))
        .expect("response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "status").as_string(),
        Some("accepted")
    );
    assert!(signal.take());
    assert!(!signal.take());

    let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_3("session.close", "{}")))
        .expect("response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let old_minor = JsonValue::parse(&close_host.handle_json(&request_v1_2("session.close", "{}")))
        .expect("response is JSON");
    assert_eq!(
        field(field(&old_minor, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}
