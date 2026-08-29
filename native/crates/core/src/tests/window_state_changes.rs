//! Contract tests for the coalesced session-window state-change read.

use super::support::*;
use crate::*;

#[test]
fn a_granted_state_change_read_returns_only_one_closed_state_or_null() {
    for (change, expected) in [
        (None, None),
        (Some(WindowState::Minimized), Some("minimized")),
        (Some(WindowState::Maximized), Some("maximized")),
        (Some(WindowState::Restored), Some("restored")),
    ] {
        let response = JsonValue::parse(
            &host_with_window_state_changes(RecordingWindowStateChanges {
                change,
                ..RecordingWindowStateChanges::default()
            })
            .handle_json(&request_v1_31("window.state.changes.read", "{}")),
        )
        .expect("response JSON is valid");
        let state = field(field(&response, "result"), "state");
        match expected {
            Some(expected) => assert_eq!(state.as_string(), Some(expected)),
            None => assert_eq!(state, &JsonValue::Null),
        }
    }
}

#[test]
fn state_change_read_has_its_own_grant_and_protocol_gate() {
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new("test.application", vec![], "test-host").expect("policy is valid"),
            HostServices::unavailable()
                .with_window_state_changes(RecordingWindowStateChanges::default()),
        )
        .handle_json(&request_v1_31("window.state.changes.read", "{}")),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_window_state_changes(RecordingWindowStateChanges::default())
            .handle_json(&request_v1_30("window.state.changes.read", "{}")),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn state_change_read_payload_is_exactly_empty() {
    for payload in [r#"{"state":"maximized"}"#, r#"{"wait":true}"#, "[]"] {
        let response = JsonValue::parse(
            &host_with_window_state_changes(RecordingWindowStateChanges::default())
                .handle_json(&request_v1_31("window.state.changes.read", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }
}

#[test]
fn state_change_read_unavailability_uses_the_safe_window_code() {
    let response = JsonValue::parse(
        &host_with_window_state_changes(RecordingWindowStateChanges {
            result: Some(WindowStateChangesServiceError::Unavailable),
            ..RecordingWindowStateChanges::default()
        })
        .handle_json(&request_v1_31("window.state.changes.read", "{}")),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("window.unavailable")
    );
}
