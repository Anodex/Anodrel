use super::support::*;
use crate::*;

#[test]
fn every_granted_window_state_reaches_only_the_state_service() {
    for (payload, expected) in [
        (r#"{"state":"minimized"}"#, WindowState::Minimized),
        (r#"{"state":"maximized"}"#, WindowState::Maximized),
        (r#"{"state":"restored"}"#, WindowState::Restored),
    ] {
        let service = RecordingWindowState::default();
        let applied = Arc::clone(&service.applied);
        let response = JsonValue::parse(
            &host_with_window_state(service)
                .handle_json(&request_v1_16("window.state.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&response, "result"), "status").as_string(),
            Some("applied"),
            "{expected:?} did not report acceptance"
        );
        assert_eq!(
            applied.lock().expect("the test mutex is usable").as_slice(),
            &[expected]
        );
    }
}

#[test]
fn window_state_needs_its_own_grant_and_protocol_version() {
    let payload = r#"{"state":"minimized"}"#;
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowTitle, Capability::UiFieldsRead],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_state(RecordingWindowState::default()),
        )
        .handle_json(&request_v1_16("window.state.set", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_window_state(RecordingWindowState::default())
            .handle_json(&request_v1_15("window.state.set", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn window_state_payload_is_exact_and_closed() {
    for payload in [
        r#"{}"#,
        r#"{"state":"fullscreen"}"#,
        r#"{"state":"minimized","target":"another-window"}"#,
        r#"{"state":"restored","bounds":{"width":1}}"#,
        r#"{"state":7}"#,
    ] {
        let response = JsonValue::parse(
            &host_with_window_state(RecordingWindowState::default())
                .handle_json(&request_v1_16("window.state.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid"),
            "{payload} was accepted"
        );
    }
}

#[test]
fn window_state_service_failures_map_to_safe_shared_codes() {
    for (error, code) in [
        (WindowStateServiceError::Unavailable, "window.unavailable"),
        (WindowStateServiceError::Busy, "window.busy"),
    ] {
        let response = JsonValue::parse(
            &host_with_window_state(RecordingWindowState {
                result: Some(error),
                ..RecordingWindowState::default()
            })
            .handle_json(&request_v1_16(
                "window.state.set",
                r#"{"state":"restored"}"#,
            )),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some(code),
            "{error:?} mapped to the wrong code"
        );
    }
}

#[test]
fn a_host_without_a_window_state_service_reports_unavailable() {
    let response = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowState],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable(),
        )
        .handle_json(&request_v1_16(
            "window.state.set",
            r#"{"state":"maximized"}"#,
        )),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("window.unavailable")
    );
}
