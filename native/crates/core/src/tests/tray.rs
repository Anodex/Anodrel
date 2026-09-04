//! Behavior checks for the bounded host-owned semantic tray menu.

use super::support::*;
use crate::*;

#[derive(Clone, Debug, Default)]
struct RecordingTray {
    replacements: Arc<Mutex<Vec<(anodrel_menu::TrayRevision, anodrel_menu::ContextMenuModel)>>>,
}

impl anodrel_menu::TrayService for RecordingTray {
    fn replace(
        &self,
        revision: anodrel_menu::TrayRevision,
        model: anodrel_menu::ContextMenuModel,
    ) -> Result<(), anodrel_menu::TrayServiceError> {
        self.replacements
            .lock()
            .expect("tray recorder lock is available")
            .push((revision, model));
        Ok(())
    }
}

fn request_v1_33(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":33}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

fn host_with_tray(service: RecordingTray) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", vec![Capability::TrayWrite], "test-host")
            .expect("test policy is valid"),
        HostServices::unavailable().with_tray(service),
    )
}

#[test]
fn a_granted_complete_tray_model_reaches_only_its_service() {
    let service = RecordingTray::default();
    let replacements = Arc::clone(&service.replacements);
    let host = host_with_tray(service);
    let payload = r#"{"items":[{"id":"window.open","label":"Open window","enabled":true}]}"#;

    let response = JsonValue::parse(&host.handle_json(&request_v1_33("tray.replace", payload)))
        .expect("response JSON is valid");
    assert_eq!(field(&response, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&response, "result"), "revision").as_string(),
        Some("1")
    );

    let replacements = replacements
        .lock()
        .expect("tray recorder lock is available");
    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].0.value(), 1);
    assert_eq!(replacements[0].1.items()[0].id().as_str(), "window.open");
    assert!(replacements[0].1.items()[0].enabled());
}

#[test]
fn a_tray_needs_its_own_grant_protocol_version_and_host_surface() {
    let payload = r#"{"items":[{"id":"window.open","label":"Open window","enabled":true}]}"#;
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::ContextMenuWrite],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_tray(RecordingTray::default()),
        )
        .handle_json(&request_v1_33("tray.replace", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_tray(RecordingTray::default()).handle_json(&format!(
            r#"{{"protocolVersion":{{"major":1,"minor":32}},"kind":"request","requestId":"request-1","operation":"tray.replace","payload":{payload}}}"#
        )),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );

    let unavailable = JsonValue::parse(
        &CoreHost::new(
            HostPolicy::new("test.application", vec![Capability::TrayWrite], "test-host")
                .expect("test policy is valid"),
        )
        .handle_json(&request_v1_33("tray.replace", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unavailable, "error"), "code").as_string(),
        Some("tray.unavailable")
    );
}

#[test]
fn a_tray_payload_is_exact_bounded_and_cannot_name_native_behavior() {
    let service = RecordingTray::default();
    let replacements = Arc::clone(&service.replacements);
    let host = host_with_tray(service);
    for payload in [
        r#"{}"#,
        r#"{"items":[]}"#,
        r#"{"items":[{"id":"window.open","label":"Open window","enabled":"true"}]}"#,
        r#"{"items":[{"id":"window.open","label":"Open window","enabled":true,"tooltip":"private"}]}"#,
        r#"{"items":[{"id":"window.open","label":"Open window","enabled":true,"coordinate":{"x":1,"y":2}}]}"#,
        r#"{"items":[{"id":"window.open","label":"Open window","enabled":true,"callback":"private"}]}"#,
    ] {
        let response = JsonValue::parse(&host.handle_json(&request_v1_33("tray.replace", payload)))
            .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid"),
            "{payload} was accepted"
        );
    }

    let oversized_label = "x".repeat(8 * 1024);
    let oversized = format!(
        r#"{{"items":[{{"id":"window.open","label":"{oversized_label}","enabled":true}}]}}"#
    );
    assert!(oversized.len() > MAX_TRAY_REPLACE_REQUEST_BYTES);
    let response = JsonValue::parse(&host.handle_json(&request_v1_33("tray.replace", &oversized)))
        .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
    assert!(
        replacements
            .lock()
            .expect("tray recorder lock is available")
            .is_empty()
    );
}

#[test]
fn tray_actions_share_ordered_revision_checked_event_delivery() {
    let mailbox = UiInputMailbox::new();
    let host = CoreHost::with_session_components_and_service_bundle(
        HostPolicy::new(
            "test.application",
            vec![Capability::TrayWrite, Capability::UiEventsRead],
            "test-host",
        )
        .expect("test policy is valid"),
        mailbox.clone(),
        SessionCloseSignal::default(),
        HostServices::unavailable().with_tray(RecordingTray::default()),
    );
    let action = anodrel_menu::MenuActionId::new("window.open").expect("test action is valid");
    let replacement = r#"{"items":[{"id":"window.open","label":"Open window","enabled":true}]}"#;
    let response = JsonValue::parse(&host.handle_json(&request_v1_33("tray.replace", replacement)))
        .expect("replacement response is JSON");
    assert_eq!(field(&response, "status").as_string(), Some("success"));
    let revision = anodrel_menu::TrayRevision::INITIAL
        .next()
        .expect("first tray revision exists");
    mailbox.push(anodrel_ui_session::TrayInputCandidate::new(
        revision,
        action.clone(),
    ));

    let read = JsonValue::parse(&host.handle_json(&request_v1_33("ui.events.read", "{}")))
        .expect("event response is JSON");
    let result = field(&read, "result");
    let JsonValue::Array(events) = field(result, "events") else {
        panic!("events is an array");
    };
    assert_eq!(events.len(), 1);
    assert_eq!(
        field(&events[0], "eventName").as_string(),
        Some("tray.action.invoked")
    );
    assert_eq!(field(&events[0], "source").as_string(), Some("native.tray"));
    assert_eq!(
        field(field(&events[0], "schemaVersion"), "minor"),
        &JsonValue::Number("33".to_owned())
    );
    assert_eq!(
        field(field(&events[0], "payload"), "trayRevision").as_string(),
        Some("1")
    );
    assert_eq!(
        field(field(&events[0], "payload"), "action").as_string(),
        Some("window.open")
    );

    let _ = host.handle_json(&request_v1_33(
        "tray.replace",
        r#"{"items":[{"id":"window.open","label":"Open window","enabled":false}]}"#,
    ));
    mailbox.push(anodrel_ui_session::TrayInputCandidate::new(
        revision, action,
    ));
    let stale = JsonValue::parse(&host.handle_json(&request_v1_33("ui.events.read", "{}")))
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
