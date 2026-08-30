use super::support::*;
use crate::*;

#[derive(Clone, Debug, Default)]
struct RecordingContextMenu {
    replacements: Arc<
        Mutex<
            Vec<(
                anodrel_menu::ContextMenuRevision,
                anodrel_menu::ContextMenuModel,
            )>,
        >,
    >,
}

impl anodrel_menu::ContextMenuService for RecordingContextMenu {
    fn replace(
        &self,
        revision: anodrel_menu::ContextMenuRevision,
        model: anodrel_menu::ContextMenuModel,
    ) -> Result<(), anodrel_menu::ContextMenuServiceError> {
        self.replacements
            .lock()
            .expect("context-menu recorder lock is available")
            .push((revision, model));
        Ok(())
    }
}

fn request_v1_32(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":32}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

fn host_with_context_menu(service: RecordingContextMenu) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::ContextMenuWrite],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_context_menu(service),
    )
}

#[test]
fn a_granted_complete_context_menu_reaches_only_its_service() {
    let service = RecordingContextMenu::default();
    let replacements = Arc::clone(&service.replacements);
    let host = host_with_context_menu(service);
    let first_payload = r#"{"items":[{"id":"document.rename","label":"Rename","enabled":true}]}"#;
    let first =
        JsonValue::parse(&host.handle_json(&request_v1_32("menu.context.replace", first_payload)))
            .expect("response JSON is valid");
    assert_eq!(field(&first, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&first, "result"), "revision").as_string(),
        Some("1")
    );

    let second = JsonValue::parse(&host.handle_json(&request_v1_32(
        "menu.context.replace",
        r#"{"items":[{"id":"document.rename","label":"Rename","enabled":false}]}"#,
    )))
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&second, "result"), "revision").as_string(),
        Some("2")
    );

    let replacements = replacements
        .lock()
        .expect("context-menu recorder lock is available");
    assert_eq!(replacements.len(), 2);
    assert_eq!(replacements[0].0.value(), 1);
    assert_eq!(
        replacements[0].1.items()[0].id().as_str(),
        "document.rename"
    );
    assert!(replacements[0].1.items()[0].enabled());
    assert_eq!(replacements[1].0.value(), 2);
    assert!(!replacements[1].1.items()[0].enabled());
}

#[test]
fn a_context_menu_needs_its_own_grant_protocol_version_and_host_surface() {
    let payload = r#"{"items":[{"id":"document.rename","label":"Rename","enabled":true}]}"#;
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new("test.application", vec![Capability::MenuWrite], "test-host")
                .expect("test policy is valid"),
            HostServices::unavailable().with_context_menu(RecordingContextMenu::default()),
        )
        .handle_json(&request_v1_32("menu.context.replace", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_context_menu(RecordingContextMenu::default()).handle_json(&format!(
            r#"{{"protocolVersion":{{"major":1,"minor":31}},"kind":"request","requestId":"request-1","operation":"menu.context.replace","payload":{payload}}}"#
        )),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );

    let unavailable = JsonValue::parse(
        &CoreHost::new(
            HostPolicy::new(
                "test.application",
                vec![Capability::ContextMenuWrite],
                "test-host",
            )
            .expect("test policy is valid"),
        )
        .handle_json(&request_v1_32("menu.context.replace", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unavailable, "error"), "code").as_string(),
        Some("menu.unavailable")
    );
}

#[test]
fn a_context_menu_payload_is_exact_bounded_and_cannot_name_native_behavior() {
    let service = RecordingContextMenu::default();
    let replacements = Arc::clone(&service.replacements);
    let host = host_with_context_menu(service);
    for payload in [
        r#"{}"#,
        r#"{"items":[]}"#,
        r#"{"items":[{"id":"document.rename","label":"Rename","enabled":"true"}]}"#,
        r#"{"items":[{"id":"document.rename","label":"Rename","enabled":true,"coordinate":{"x":1,"y":2}}]}"#,
        r#"{"items":[{"id":"document.rename","label":"Rename","enabled":true,"selection":"private"}]}"#,
        r#"{"items":[{"id":"document.rename","label":"Rename","enabled":true,"shortcut":"Ctrl+R"}]}"#,
        r#"{"items":[{"id":"document.rename","label":"Rename","enabled":true},{"id":"document.rename","label":"Again","enabled":true}]}"#,
    ] {
        let response =
            JsonValue::parse(&host.handle_json(&request_v1_32("menu.context.replace", payload)))
                .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid"),
            "{payload} was accepted"
        );
    }

    let oversized_label = "x".repeat(8 * 1024);
    let oversized = format!(
        r#"{{"items":[{{"id":"document.rename","label":"{oversized_label}","enabled":true}}]}}"#
    );
    assert!(oversized.len() > MAX_CONTEXT_MENU_REPLACE_REQUEST_BYTES);
    let response =
        JsonValue::parse(&host.handle_json(&request_v1_32("menu.context.replace", &oversized)))
            .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
    assert!(
        replacements
            .lock()
            .expect("context-menu recorder lock is available")
            .is_empty()
    );
}

#[test]
fn context_menu_actions_share_ordered_revision_checked_event_delivery() {
    let mailbox = UiInputMailbox::new();
    let host = CoreHost::with_session_components_and_service_bundle(
        HostPolicy::new(
            "test.application",
            vec![Capability::ContextMenuWrite, Capability::UiEventsRead],
            "test-host",
        )
        .expect("test policy is valid"),
        mailbox.clone(),
        SessionCloseSignal::default(),
        HostServices::unavailable().with_context_menu(RecordingContextMenu::default()),
    );
    let response = JsonValue::parse(&host.handle_json(&request_v1_32(
        "menu.context.replace",
        r#"{"items":[{"id":"document.rename","label":"Rename","enabled":true}]}"#,
    )))
    .expect("replacement response is JSON");
    assert_eq!(field(&response, "status").as_string(), Some("success"));
    let revision = anodrel_menu::ContextMenuRevision::INITIAL
        .next()
        .expect("first context-menu revision exists");
    let action = anodrel_menu::MenuActionId::new("document.rename")
        .expect("test context-menu action is valid");
    mailbox.push(anodrel_ui_session::ContextMenuInputCandidate::new(
        revision,
        action.clone(),
    ));

    let read = JsonValue::parse(&host.handle_json(&request_v1_32("ui.events.read", "{}")))
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
        Some("menu.context.action.invoked")
    );
    assert_eq!(
        field(&events[0], "source").as_string(),
        Some("native.context_menu")
    );
    assert_eq!(
        field(field(&events[0], "schemaVersion"), "minor"),
        &JsonValue::Number("32".to_owned())
    );
    assert_eq!(
        field(field(&events[0], "payload"), "contextMenuRevision").as_string(),
        Some("1")
    );
    assert_eq!(
        field(field(&events[0], "payload"), "action").as_string(),
        Some("document.rename")
    );

    let _ = host.handle_json(&request_v1_32(
        "menu.context.replace",
        r#"{"items":[{"id":"document.rename","label":"Rename","enabled":false}]}"#,
    ));
    mailbox.push(anodrel_ui_session::ContextMenuInputCandidate::new(
        revision, action,
    ));
    let stale = JsonValue::parse(&host.handle_json(&request_v1_32("ui.events.read", "{}")))
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
