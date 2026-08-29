//! Contract tests for Protocol 1.30 pull-only session window-state observation.

use super::support::*;
use crate::*;

#[test]
fn a_granted_window_state_read_returns_only_the_closed_snapshot() {
    for (state, expected) in [
        (WindowState::Minimized, "minimized"),
        (WindowState::Maximized, "maximized"),
        (WindowState::Restored, "restored"),
    ] {
        let response = JsonValue::parse(
            &host_with_window_state_read(RecordingWindowStateRead {
                state,
                ..RecordingWindowStateRead::default()
            })
            .handle_json(&request_v1_30("window.state.get", "{}")),
        )
        .expect("response JSON is valid");
        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&response, "result"), "state").as_string(),
            Some(expected)
        );
    }
}

#[test]
fn state_read_has_its_own_grant_and_protocol_gate() {
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowState],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_state_read(RecordingWindowStateRead::default()),
        )
        .handle_json(&request_v1_30("window.state.get", "{}")),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_window_state_read(RecordingWindowStateRead::default())
            .handle_json(&request_v1_29("window.state.get", "{}")),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn state_read_payload_is_exactly_empty() {
    for payload in [
        r#"{"target":"another-window"}"#,
        r#"{"state":"maximized"}"#,
        r#"[]"#,
    ] {
        let response = JsonValue::parse(
            &host_with_window_state_read(RecordingWindowStateRead::default())
                .handle_json(&request_v1_30("window.state.get", payload)),
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
fn state_read_failures_use_safe_shared_window_codes() {
    for (error, code) in [
        (
            WindowStateReadServiceError::Unavailable,
            "window.unavailable",
        ),
        (WindowStateReadServiceError::Busy, "window.busy"),
    ] {
        let response = JsonValue::parse(
            &host_with_window_state_read(RecordingWindowStateRead {
                result: Some(error),
                ..RecordingWindowStateRead::default()
            })
            .handle_json(&request_v1_30("window.state.get", "{}")),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some(code)
        );
    }
}
