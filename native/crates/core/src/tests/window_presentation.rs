use super::support::*;
use crate::*;

#[test]
fn every_granted_window_fullscreen_mode_reaches_only_the_fullscreen_service() {
    for (payload, expected) in [
        (r#"{"mode":"fullscreen"}"#, WindowFullscreenMode::Fullscreen),
        (r#"{"mode":"windowed"}"#, WindowFullscreenMode::Windowed),
    ] {
        let service = RecordingWindowFullscreen::default();
        let applied = Arc::clone(&service.applied);
        let response = JsonValue::parse(
            &host_with_window_fullscreen(service)
                .handle_json(&request_v1_21("window.fullscreen.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&response, "result"), "status").as_string(),
            Some("applied")
        );
        assert_eq!(
            applied.lock().expect("the test mutex is usable").as_slice(),
            &[expected]
        );
    }
}

#[test]
fn window_fullscreen_needs_its_own_grant_and_protocol_version() {
    let payload = r#"{"mode":"fullscreen"}"#;
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowFocus, Capability::WindowState],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable()
                .with_window_fullscreen(RecordingWindowFullscreen::default()),
        )
        .handle_json(&request_v1_21("window.fullscreen.set", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_window_fullscreen(RecordingWindowFullscreen::default())
            .handle_json(&request_v1_20("window.fullscreen.set", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn window_fullscreen_payload_is_exact_and_closed() {
    for payload in [
        r#"{}"#,
        r#"{"mode":"exclusive"}"#,
        r#"{"mode":"fullscreen","monitor":"other"}"#,
        r#"{"mode":"windowed","bounds":{"width":1}}"#,
        r#"{"mode":true}"#,
    ] {
        let response = JsonValue::parse(
            &host_with_window_fullscreen(RecordingWindowFullscreen::default())
                .handle_json(&request_v1_21("window.fullscreen.set", payload)),
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
fn window_fullscreen_service_failures_map_to_safe_shared_codes() {
    for (error, code) in [
        (
            WindowFullscreenServiceError::Unavailable,
            "window.unavailable",
        ),
        (WindowFullscreenServiceError::Busy, "window.busy"),
    ] {
        let response = JsonValue::parse(
            &host_with_window_fullscreen(RecordingWindowFullscreen {
                result: Some(error),
                ..RecordingWindowFullscreen::default()
            })
            .handle_json(&request_v1_21(
                "window.fullscreen.set",
                r#"{"mode":"fullscreen"}"#,
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
fn a_host_without_a_window_fullscreen_service_reports_unavailable() {
    let response = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowFullscreen],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable(),
        )
        .handle_json(&request_v1_21(
            "window.fullscreen.set",
            r#"{"mode":"windowed"}"#,
        )),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("window.unavailable")
    );
}

#[test]
fn a_granted_window_size_reaches_only_the_size_service() {
    let service = RecordingWindowSize::default();
    let applied = Arc::clone(&service.applied);
    let response = JsonValue::parse(&host_with_window_size(service).handle_json(&request_v1_23(
        "window.size.set",
        r#"{"width":800,"height":600}"#,
    )))
    .expect("response JSON is valid");

    assert_eq!(field(&response, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&response, "result"), "status").as_string(),
        Some("applied")
    );
    assert_eq!(
        applied.lock().expect("the test mutex is usable").as_slice(),
        &[WindowSize::new(800, 600).expect("fixture size is valid")]
    );
}

#[test]
fn window_size_needs_its_own_grant_and_protocol_version() {
    let payload = r#"{"width":800,"height":600}"#;
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowFullscreen, Capability::WindowState],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_size(RecordingWindowSize::default()),
        )
        .handle_json(&request_v1_23("window.size.set", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_window_size(RecordingWindowSize::default())
            .handle_json(&request_v1_22("window.size.set", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn window_size_payload_is_exact_and_bounded() {
    for payload in [
        r#"{}"#,
        r#"{"width":319,"height":600}"#,
        r#"{"width":800,"height":239}"#,
        r#"{"width":3841,"height":600}"#,
        r#"{"width":800,"height":2161}"#,
        r#"{"width":800.0,"height":600}"#,
        r#"{"width":800,"height":600,"x":0}"#,
        r#"{"width":800,"height":600,"monitor":"other"}"#,
    ] {
        let response = JsonValue::parse(
            &host_with_window_size(RecordingWindowSize::default())
                .handle_json(&request_v1_23("window.size.set", payload)),
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
fn window_size_service_failures_map_to_safe_shared_codes() {
    for (error, code) in [
        (WindowSizeServiceError::Unavailable, "window.unavailable"),
        (WindowSizeServiceError::Busy, "window.busy"),
    ] {
        let response = JsonValue::parse(
            &host_with_window_size(RecordingWindowSize {
                result: Some(error),
                ..RecordingWindowSize::default()
            })
            .handle_json(&request_v1_23(
                "window.size.set",
                r#"{"width":800,"height":600}"#,
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
fn a_host_without_a_window_size_service_reports_unavailable() {
    let response = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowSize],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable(),
        )
        .handle_json(&request_v1_23(
            "window.size.set",
            r#"{"width":800,"height":600}"#,
        )),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("window.unavailable")
    );
}
