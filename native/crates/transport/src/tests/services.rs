//! Authenticated transport coverage for injected host services.

use super::*;

#[test]
fn routes_a_granted_credential_read_to_the_injected_service_after_authentication() {
    let mut transport = TransportSession::with_credential_service(
        HostPolicy::new(
            "test.application",
            vec![Capability::CredentialRead],
            "test-host",
        )
        .expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
        FixedCredentialService,
    );
    authenticate(&mut transport);

    let response = transport
        .receive(
            &encode_json(&credential_request(
                "credential.read",
                r#"{"name":"refresh-token"}"#,
            ))
            .expect("credential request encodes"),
        )
        .expect("credential response is returned");
    let response = decode_response(&response[0]);
    let result = response
        .as_object()
        .and_then(|fields| fields.get("result"))
        .and_then(JsonValue::as_object)
        .expect("credential response has a result");
    assert_eq!(
        result.get("status").and_then(JsonValue::as_string),
        Some("found")
    );
    assert_eq!(
        result.get("secret").and_then(JsonValue::as_string),
        Some("00aaff")
    );
}

#[test]
fn routes_granted_clipboard_operations_to_the_injected_service_after_authentication() {
    let mut transport = TransportSession::with_session_components_and_clipboard(
        HostPolicy::new(
            "test.application",
            vec![Capability::ClipboardRead, Capability::ClipboardWrite],
            "test-host",
        )
        .expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text("before"),
    );
    authenticate(&mut transport);

    let read = transport
        .receive(&encode_json(&clipboard_request("clipboard.read", "{}")).expect("request encodes"))
        .expect("read response is returned");
    assert_eq!(
        decode_response(&read[0])
            .as_object()
            .expect("response object")["result"]
            .as_object()
            .expect("result object")["text"]
            .as_string(),
        Some("before")
    );

    let write = transport
        .receive(
            &encode_json(&clipboard_request("clipboard.write", r#"{"text":"after"}"#))
                .expect("request encodes"),
        )
        .expect("write response is returned");
    assert_eq!(
        decode_response(&write[0])
            .as_object()
            .expect("response object")["result"]
            .as_object()
            .expect("result object")["status"]
            .as_string(),
        Some("written")
    );
}

#[test]
fn routes_a_granted_external_link_to_the_injected_service_after_authentication() {
    let mut transport = TransportSession::with_session_components_and_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::ExternalOpen],
            "test-host",
        )
        .expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text("unused"),
        RecordingExternalLinks,
    );
    authenticate(&mut transport);

    let response = transport
        .receive(
            &encode_json(&external_open_request("https://docs.anodrel.dev/guide"))
                .expect("request encodes"),
        )
        .expect("external response is returned");
    assert_eq!(
        decode_response(&response[0])
            .as_object()
            .expect("response object")["result"]
            .as_object()
            .expect("result object")["status"]
            .as_string(),
        Some("opened")
    );
}
