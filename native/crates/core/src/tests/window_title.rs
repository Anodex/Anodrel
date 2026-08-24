use super::support::*;
use crate::*;

#[test]
fn a_granted_window_title_reaches_the_service_unchanged() {
    let service = RecordingWindowTitle::default();
    let response = JsonValue::parse(&host_with_window_title(service).handle_json(&request_v1_14(
        "window.title.set",
        r#"{"title":"Quarterly Report.pdf"}"#,
    )))
    .expect("response JSON is valid");

    assert_eq!(field(&response, "status").as_string(), Some("success"));
    // Acceptance only. The composed caption is deliberately not returned:
    // it would hand the application the host's framing format to probe.
    assert_eq!(
        field(field(&response, "result"), "status").as_string(),
        Some("applied")
    );
}

#[test]
fn a_window_title_needs_its_own_grant_and_its_own_protocol_version() {
    let payload = r#"{"title":"Report"}"#;

    // Held every other grant, but not this one.
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::NotificationShow, Capability::DiagnosticsRead],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_title(RecordingWindowTitle::default()),
        )
        .handle_json(&request_v1_14("window.title.set", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    // Granted, but asked for at a protocol minor that predates it.
    let unsupported = JsonValue::parse(
        &host_with_window_title(RecordingWindowTitle::default())
            .handle_json(&request_v1_13("window.title.set", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn a_window_title_payload_accepts_exactly_one_title_field() {
    // An extra field is a mismatch rather than something to ignore, so a
    // future target, identifier, position, or size cannot be smuggled past
    // this version. That refusal is what keeps the capability un-aimable.
    for payload in [
        r#"{}"#,
        r#"{"title":"Report","target":"other-window"}"#,
        r#"{"title":"Report","windowId":2}"#,
        r#"{"caption":"Report"}"#,
        r#"{"title":7}"#,
    ] {
        let response = JsonValue::parse(
            &host_with_window_title(RecordingWindowTitle::default())
                .handle_json(&request_v1_14("window.title.set", payload)),
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
fn a_rejected_window_title_never_echoes_the_text_it_refused() {
    // The marker is what a leak would look like: text refused for being
    // unsafe to display must not be repeated in an error that reaches logs.
    let marker = "MarkerZQX";
    let response = host_with_window_title(RecordingWindowTitle::default()).handle_json(
        &request_v1_14("window.title.set", &format!(r#"{{"title":"{marker}\n"}}"#)),
    );
    let parsed = JsonValue::parse(&response).expect("response JSON is valid");
    assert_eq!(
        field(field(&parsed, "error"), "code").as_string(),
        Some("window.title_invalid")
    );
    assert!(!response.contains(marker), "the refused title was echoed");
}

#[test]
fn window_title_service_failures_map_to_their_own_codes() {
    for (error, code) in [
        (WindowTitleServiceError::Unavailable, "window.unavailable"),
        (WindowTitleServiceError::Busy, "window.busy"),
    ] {
        let service = RecordingWindowTitle {
            result: Some(error),
            ..RecordingWindowTitle::default()
        };
        let response = JsonValue::parse(
            &host_with_window_title(service)
                .handle_json(&request_v1_14("window.title.set", r#"{"title":"Report"}"#)),
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
fn a_host_without_a_window_title_service_reports_unavailable() {
    let response = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowTitle],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable(),
        )
        .handle_json(&request_v1_14("window.title.set", r#"{"title":"Report"}"#)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("window.unavailable")
    );
}
