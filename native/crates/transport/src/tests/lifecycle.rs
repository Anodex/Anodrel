//! Authentication, close, and semantic-event transport tests.

use super::*;

#[test]
fn returns_current_semantic_ui_actions_through_an_authenticated_session() {
    let documents = UiDocumentMailbox::new();
    let inputs = UiInputMailbox::new();
    let mut transport = TransportSession::with_ui_mailboxes(
        HostPolicy::new(
            "test.application",
            vec![Capability::UiDocumentWrite, Capability::UiEventsRead],
            "test-host",
        )
        .expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
        documents.clone(),
        inputs.clone(),
    );
    authenticate(&mut transport);
    let replacement = transport
        .receive(&encode_json(&ui_action_document_request()).expect("request encodes"))
        .expect("document replacement succeeds");
    assert_eq!(
        decode_response(&replacement[0])
            .as_object()
            .expect("response object")["status"]
            .as_string(),
        Some("success")
    );

    let revision = documents
        .take()
        .expect("accepted document is published")
        .revision();
    inputs.push(anodrel_ui_session::UiInputCandidate::new(
        revision,
        UiEvent::ActionInvoked(ElementId::new("root").expect("test ID is valid")),
    ));
    let response = transport
        .receive(&encode_json(&ui_events_read_request()).expect("request encodes"))
        .expect("event read succeeds");
    let decoded = decode_response(&response[0]);
    let result = &decoded.as_object().expect("response object")["result"];
    let JsonValue::Array(events) = &result.as_object().expect("result object")["events"] else {
        panic!("events array");
    };
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].as_object().expect("event object")["eventName"].as_string(),
        Some("ui.action.invoked")
    );
}

#[test]
fn returns_a_close_response_before_the_host_consumes_its_session_signal() {
    let close_signal = SessionCloseSignal::default();
    let mut transport = TransportSession::with_session_components(
        HostPolicy::new(
            "test.application",
            vec![Capability::SessionClose],
            "test-host",
        )
        .expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        close_signal.clone(),
    );
    authenticate(&mut transport);

    let response = transport
        .receive(&encode_json(&session_close_request()).expect("request encodes"))
        .expect("close response is returned");
    let decoded = decode_response(&response[0]);
    assert_eq!(
        decoded.as_object().expect("response object")["status"].as_string(),
        Some("success")
    );
    assert_eq!(
        decoded.as_object().expect("response object")["result"]
            .as_object()
            .expect("result object")["status"]
            .as_string(),
        Some("accepted")
    );
    assert!(close_signal.take());
}

#[test]
fn rejects_wrong_tokens_and_second_authentication_attempts() {
    let mut transport = session(vec![]);
    let error = transport
        .receive(
            &encode_json(
                &authentication_message(
                    SESSION_ID,
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                )
                .expect("authentication is structurally valid"),
            )
            .expect("frame encodes"),
        )
        .expect_err("wrong token is rejected");
    assert!(matches!(error, TransportError::AuthenticationFailed));

    let mut authenticated = session(vec![]);
    authenticate(&mut authenticated);
    assert!(matches!(
        authenticated.receive(
            &encode_json(
                &authentication_message(SESSION_ID, TOKEN).expect("authentication is valid")
            )
            .expect("frame encodes")
        ),
        Err(TransportError::AuthenticationFailed)
    ));
}

#[test]
fn exposes_wire_failures_without_attempting_resynchronization() {
    let mut transport = session(vec![]);
    let error = transport
        .receive(b"not a frame!")
        .expect_err("bad magic is rejected");
    assert!(matches!(
        error,
        TransportError::Wire(WireError::InvalidMagic)
    ));
}
