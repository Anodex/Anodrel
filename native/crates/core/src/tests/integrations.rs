use super::support::*;
use crate::*;

#[test]
fn clipboard_operations_are_separate_bounded_and_capability_checked() {
    let clipboard_host = clipboard_host(
        vec![Capability::ClipboardRead, Capability::ClipboardWrite],
        MemoryClipboard::with_text(Some("before")),
    );

    let read = JsonValue::parse(&clipboard_host.handle_json(&request_v1_5("clipboard.read", "{}")))
        .expect("clipboard read response is JSON");
    assert_eq!(field(&read, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&read, "result"), "status").as_string(),
        Some("text")
    );
    assert_eq!(
        field(field(&read, "result"), "text").as_string(),
        Some("before")
    );

    let write = JsonValue::parse(
        &clipboard_host.handle_json(&request_v1_5("clipboard.write", r#"{"text":"after"}"#)),
    )
    .expect("clipboard write response is JSON");
    assert_eq!(
        field(field(&write, "result"), "status").as_string(),
        Some("written")
    );

    let updated =
        JsonValue::parse(&clipboard_host.handle_json(&request_v1_5("clipboard.read", "{}")))
            .expect("updated clipboard read response is JSON");
    assert_eq!(
        field(field(&updated, "result"), "text").as_string(),
        Some("after")
    );

    let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_5("clipboard.read", "{}")))
        .expect("denied clipboard response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let oversized = object([(
        "text",
        JsonValue::String("x".repeat(MAX_CLIPBOARD_TEXT_REQUEST_BYTES + 1)),
    )])
    .to_json();
    let rejected =
        JsonValue::parse(&clipboard_host.handle_json(&request_v1_5("clipboard.write", &oversized)))
            .expect("oversized clipboard response is JSON");
    assert_eq!(
        field(field(&rejected, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
}

#[test]
fn clipboard_service_failures_have_safe_stable_protocol_codes() {
    let host = clipboard_host(
        vec![Capability::ClipboardRead, Capability::ClipboardWrite],
        FailingClipboard(ClipboardServiceError::StoredTextInvalid),
    );
    let response = JsonValue::parse(&host.handle_json(&request_v1_5("clipboard.read", "{}")))
        .expect("clipboard failure response is JSON");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("clipboard.text_invalid")
    );
    assert!(
        field(field(&response, "error"), "message")
            .as_string()
            .is_some_and(|message| !message.contains("before"))
    );
}

#[test]
fn external_open_requires_its_own_grant_and_validated_https_url() {
    let external_host = external_host(
        vec![Capability::ExternalOpen],
        RecordingExternalLinks::default(),
    );
    let accepted = JsonValue::parse(&external_host.handle_json(&request_v1_6(
        "external.open",
        r#"{"url":"https://docs.anodrel.dev/guide"}"#,
    )))
    .expect("external open response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "status").as_string(),
        Some("opened")
    );

    let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_6(
        "external.open",
        r#"{"url":"https://docs.anodrel.dev/guide"}"#,
    )))
    .expect("denied external open response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let invalid = JsonValue::parse(&external_host.handle_json(&request_v1_6(
        "external.open",
        r#"{"url":"file:///C:/private.txt"}"#,
    )))
    .expect("invalid external open response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
}

#[test]
fn external_service_failure_never_exposes_a_url_or_native_status() {
    let host = external_host(vec![Capability::ExternalOpen], FailingExternalLinks);
    let response = JsonValue::parse(&host.handle_json(&request_v1_6(
        "external.open",
        r#"{"url":"https://docs.anodrel.dev/private"}"#,
    )))
    .expect("external failure response is JSON");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("external.unavailable")
    );
    assert!(
        field(field(&response, "error"), "message")
            .as_string()
            .is_some_and(|message| !message.contains("private"))
    );
}

#[test]
fn network_text_fetch_is_separately_granted_and_returns_only_status_and_text() {
    let service = RecordingNetwork::responding(201, "created");
    let requested = Arc::clone(&service.requested);
    let host = network_host(vec![Capability::NetworkFetch], service);
    let response = JsonValue::parse(&host.handle_json(&request_v1_19(
        "network.fetch_text",
        r#"{"url":"https://Api.Example.test:8443/v1/status?format=text"}"#,
    )))
    .expect("network response is JSON");
    assert_eq!(field(&response, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&response, "result"), "statusCode"),
        &JsonValue::Number("201".to_owned())
    );
    assert_eq!(
        field(field(&response, "result"), "text").as_string(),
        Some("created")
    );
    let requested = requested
        .lock()
        .expect("network recorder lock is available");
    assert_eq!(requested.len(), 1);
    assert_eq!(
        requested[0].as_str(),
        "https://Api.Example.test:8443/v1/status?format=text"
    );
    assert_eq!(requested[0].hostname(), "api.example.test");
    assert_eq!(requested[0].port(), 8443);
}

#[test]
fn network_text_fetch_requires_its_protocol_version_grant_and_host_service() {
    let payload = r#"{"url":"https://api.example.test/status"}"#;
    let denied = JsonValue::parse(
        &network_host(vec![], RecordingNetwork::responding(200, "healthy"))
            .handle_json(&request_v1_19("network.fetch_text", payload)),
    )
    .expect("denied network response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );
    assert_eq!(
        field(field(&denied, "error"), "details")
            .as_object()
            .and_then(|details| details.get("capability"))
            .and_then(JsonValue::as_string),
        Some("network.fetch")
    );

    let unsupported = JsonValue::parse(
        &network_host(
            vec![Capability::NetworkFetch],
            RecordingNetwork::responding(200, "healthy"),
        )
        .handle_json(&request_v1_18("network.fetch_text", payload)),
    )
    .expect("old-version network response is JSON");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );

    let unavailable = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::NetworkFetch],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable(),
        )
        .handle_json(&request_v1_19("network.fetch_text", payload)),
    )
    .expect("unavailable network response is JSON");
    assert_eq!(
        field(field(&unavailable, "error"), "code").as_string(),
        Some("network.unavailable")
    );
}

#[test]
fn network_text_fetch_rejects_unrepresentable_values_and_never_echoes_them() {
    let service = RecordingNetwork::responding(200, "healthy");
    let requested = Arc::clone(&service.requested);
    let host = network_host(vec![Capability::NetworkFetch], service);
    let marker = "PrivateNetworkMarker";
    for payload in [
        r#"{}"#,
        r#"{"url":"https://api.example.test/status","header":"secret"}"#,
        r#"{"url":"https://127.0.0.1/status"}"#,
        &format!(r#"{{"url":"https://api.example.test/{marker}%1"}}"#),
    ] {
        let response = host.handle_json(&request_v1_19("network.fetch_text", payload));
        let parsed = JsonValue::parse(&response).expect("invalid network response is JSON");
        assert_eq!(
            field(field(&parsed, "error"), "code").as_string(),
            Some("request.payload_invalid"),
            "{payload} was accepted"
        );
        assert!(
            !response.contains(marker),
            "refused URL leaked into the response"
        );
    }
    assert!(
        requested
            .lock()
            .expect("network recorder lock is available")
            .is_empty(),
        "a rejected request reached the service"
    );

    for (error, code) in [
        (NetworkTextServiceError::Unavailable, "network.unavailable"),
        (
            NetworkTextServiceError::ResponseInvalid,
            "network.response_invalid",
        ),
    ] {
        let response = JsonValue::parse(
            &network_host(
                vec![Capability::NetworkFetch],
                RecordingNetwork::failing(error),
            )
            .handle_json(&request_v1_19(
                "network.fetch_text",
                r#"{"url":"https://api.example.test/status"}"#,
            )),
        )
        .expect("failed network response is JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some(code),
            "{error:?} mapped to the wrong protocol error"
        );
    }
}
