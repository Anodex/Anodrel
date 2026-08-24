use super::support::*;
use crate::*;

#[test]
fn a_granted_window_focus_request_reaches_only_the_focus_service() {
    let service = RecordingWindowFocus::default();
    let requested = Arc::clone(&service.requested);
    let response = JsonValue::parse(
        &host_with_window_focus(service)
            .handle_json(&request_v1_20("window.focus.request", r#"{}"#)),
    )
    .expect("response JSON is valid");

    assert_eq!(field(&response, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&response, "result"), "status").as_string(),
        Some("requested")
    );
    assert_eq!(*requested.lock().expect("the test mutex is usable"), 1);
}

#[test]
fn window_focus_needs_its_own_grant_and_protocol_version() {
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowState, Capability::WindowTitle],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_focus(RecordingWindowFocus::default()),
        )
        .handle_json(&request_v1_20("window.focus.request", r#"{}"#)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_window_focus(RecordingWindowFocus::default())
            .handle_json(&request_v1_19("window.focus.request", r#"{}"#)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn window_focus_payload_is_exact_and_untargetable() {
    for payload in [
        r#"null"#,
        r#"{"target":"another-window"}"#,
        r#"{"handle":7}"#,
        r#"{"retry":true}"#,
        r#"{"input":"click"}"#,
    ] {
        let response = JsonValue::parse(
            &host_with_window_focus(RecordingWindowFocus::default())
                .handle_json(&request_v1_20("window.focus.request", payload)),
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
fn window_focus_service_failures_map_to_safe_shared_codes() {
    for (error, code) in [
        (WindowFocusServiceError::Unavailable, "window.unavailable"),
        (WindowFocusServiceError::Busy, "window.busy"),
    ] {
        let response = JsonValue::parse(
            &host_with_window_focus(RecordingWindowFocus {
                result: Some(error),
                ..RecordingWindowFocus::default()
            })
            .handle_json(&request_v1_20("window.focus.request", r#"{}"#)),
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
fn a_host_without_a_window_focus_service_reports_unavailable() {
    let response = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowFocus],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable(),
        )
        .handle_json(&request_v1_20("window.focus.request", r#"{}"#)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("window.unavailable")
    );
}
