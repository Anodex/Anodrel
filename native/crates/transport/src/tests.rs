//! Focused verification for authenticated transport sessions and cancellation.

use std::cell::RefCell;

use anodrel_clipboard::{ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText};
use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
use anodrel_ui::{ElementId, UiEvent};
use anodrel_ui_session::UiWindowGroup;
use anodrel_window::WindowTitleProposal;

use anodrel_protocol::{Capability, JsonValue};
use anodrel_wire::{FrameDecoder, WireError, encode_json};

use super::*;

mod services;

const SESSION_ID: &str = "test-session";
const TOKEN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn session(grants: Vec<Capability>) -> TransportSession {
    TransportSession::new(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
    )
}

fn request(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":0}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

fn cancellable_request(operation: &str, payload: &str, cancellation_id: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":0}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload},"cancellationId":"{cancellation_id}"}}"#
    )
}

fn cancellation(cancellation_id: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":0}},"kind":"cancel","cancellationId":"{cancellation_id}"}}"#
    )
}

fn ui_document_request() -> String {
    r#"{"protocolVersion":{"major":1,"minor":1},"kind":"request","requestId":"ui-document","operation":"ui.document.replace","payload":{"document":"{\"format\":\"anodrel.ui.document.v1\",\"root\":{\"id\":\"root\",\"kind\":\"text\",\"value\":\"Hello\",\"fontSize\":16,\"tone\":\"primary\"}}"}}"#.to_owned()
}

fn ui_action_document_request() -> String {
    r#"{"protocolVersion":{"major":1,"minor":1},"kind":"request","requestId":"ui-document","operation":"ui.document.replace","payload":{"document":"{\"format\":\"anodrel.ui.document.v1\",\"root\":{\"id\":\"root\",\"kind\":\"action\",\"label\":\"Continue\",\"fontSize\":16,\"enabled\":true,\"tone\":\"accent\"}}"}}"#.to_owned()
}

fn ui_events_read_request() -> String {
    r#"{"protocolVersion":{"major":1,"minor":2},"kind":"request","requestId":"ui-events","operation":"ui.events.read","payload":{}}"#.to_owned()
}

fn session_close_request() -> String {
    r#"{"protocolVersion":{"major":1,"minor":3},"kind":"request","requestId":"session-close","operation":"session.close","payload":{}}"#.to_owned()
}

fn clipboard_request(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":5}},"kind":"request","requestId":"clipboard","operation":"{operation}","payload":{payload}}}"#
    )
}

fn external_open_request(url: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":6}},"kind":"request","requestId":"external","operation":"external.open","payload":{{"url":"{url}"}}}}"#
    )
}

fn credential_request(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":12}},"kind":"request","requestId":"credential","operation":"{operation}","payload":{payload}}}"#
    )
}

#[derive(Debug)]
struct MemoryClipboard(RefCell<Option<ClipboardText>>);

impl MemoryClipboard {
    fn with_text(text: &str) -> Self {
        Self(RefCell::new(Some(
            ClipboardText::new(text).expect("fixture text is valid"),
        )))
    }
}

impl ClipboardService for MemoryClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        Ok(self
            .0
            .borrow()
            .clone()
            .map(ClipboardRead::Text)
            .unwrap_or(ClipboardRead::NoText))
    }

    fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        *self.0.borrow_mut() = Some(text.clone());
        Ok(())
    }
}

#[derive(Debug, Default)]
struct RecordingExternalLinks;

impl ExternalLinkService for RecordingExternalLinks {
    fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        Ok(())
    }
}

#[derive(Debug)]
struct FixedCredentialService;

impl CredentialService for FixedCredentialService {
    fn read(&self, _name: &CredentialName) -> Result<Secret, CredentialServiceError> {
        Secret::new(vec![0, 0xaa, 0xff]).map_err(|_| CredentialServiceError::StoredSecretInvalid)
    }

    fn write(
        &self,
        _name: &CredentialName,
        _secret: &Secret,
    ) -> Result<(), CredentialServiceError> {
        Ok(())
    }

    fn delete(&self, _name: &CredentialName) -> Result<bool, CredentialServiceError> {
        Ok(false)
    }
}

fn decode_response(frame: &[u8]) -> JsonValue {
    let messages = FrameDecoder::new()
        .push(frame)
        .expect("response frame decodes");
    assert_eq!(messages.len(), 1);
    JsonValue::parse(&messages[0]).expect("response is JSON")
}

fn authenticate(transport: &mut TransportSession) {
    let response = transport
        .receive(
            &encode_json(
                &authentication_message(SESSION_ID, TOKEN).expect("authentication is valid"),
            )
            .expect("authentication frame encodes"),
        )
        .expect("authentication succeeds");
    assert_eq!(
        decode_response(&response[0])
            .as_object()
            .expect("response object")["kind"]
            .as_string(),
        Some(AUTHENTICATED_KIND)
    );
}

#[test]
fn requires_authentication_before_public_requests() {
    let mut transport = session(vec![]);
    let error = transport
        .receive(&encode_json(&request("platform.health", "{}")).expect("request encodes"))
        .expect_err("unauthenticated request is rejected");
    assert!(matches!(error, TransportError::AuthenticationFailed));
    assert!(matches!(
        transport.receive(&[]),
        Err(TransportError::SessionClosed)
    ));
}

#[test]
fn handles_a_fragmented_authenticated_health_request() {
    let frame = encode_json(&request("platform.health", "{}")).expect("request encodes");
    let mut transport = session(vec![Capability::DiagnosticsRead]);
    authenticate(&mut transport);
    assert!(
        transport
            .receive(&frame[..9])
            .expect("fragment is valid")
            .is_empty()
    );

    let responses = transport.receive(&frame[9..]).expect("frame completes");
    let response = decode_response(&responses[0]);
    assert_eq!(
        response.as_object().expect("response object")["status"].as_string(),
        Some("success")
    );
    assert_eq!(
        response.as_object().expect("response object")["result"]
            .as_object()
            .expect("result object")["status"]
            .as_string(),
        Some("ready")
    );
}

#[test]
fn cancels_only_a_request_that_has_not_started() {
    let mut transport = session(vec![Capability::DiagnosticsRead]);
    authenticate(&mut transport);

    let control = transport
        .receive(&encode_json(&cancellation("stop-before-start")).expect("control encodes"))
        .expect("cancellation control is accepted");
    assert!(control.is_empty());

    let response = transport
        .receive(
            &encode_json(&cancellable_request(
                "platform.health",
                "{}",
                "stop-before-start",
            ))
            .expect("request encodes"),
        )
        .expect("cancelled request returns a response");
    let response = decode_response(&response[0]);
    assert_eq!(
        response.as_object().expect("response object")["error"]
            .as_object()
            .expect("error object")["code"]
            .as_string(),
        Some("request.cancelled")
    );

    let completed = transport
        .receive(
            &encode_json(&cancellable_request("platform.health", "{}", "too-late"))
                .expect("request encodes"),
        )
        .expect("request completes before cancellation");
    assert_eq!(
        decode_response(&completed[0])
            .as_object()
            .expect("response object")["status"]
            .as_string(),
        Some("success")
    );
    assert!(
        transport
            .receive(&encode_json(&cancellation("too-late")).expect("control encodes"))
            .expect("late control is accepted")
            .is_empty()
    );
}

#[test]
fn closes_when_cancellation_only_traffic_reaches_the_bounded_limit() {
    let mut transport = session(vec![]);
    authenticate(&mut transport);
    for index in 0..MAX_PENDING_CANCELLATIONS {
        assert!(
            transport
                .receive(
                    &encode_json(&cancellation(&format!("pending-{index}")))
                        .expect("control encodes"),
                )
                .expect("control below limit is accepted")
                .is_empty()
        );
    }
    assert!(matches!(
        transport.receive(&encode_json(&cancellation("one-too-many")).expect("control encodes"),),
        Err(TransportError::CancellationLimitReached)
    ));
    assert!(matches!(
        transport.receive(&[]),
        Err(TransportError::SessionClosed)
    ));
}

#[test]
fn closes_on_a_malformed_cancellation_control() {
    let mut transport = session(vec![]);
    authenticate(&mut transport);
    assert!(matches!(
        transport.receive(
            &encode_json(
                r#"{"protocolVersion":{"major":1,"minor":0},"kind":"cancel","cancellationId":""}"#
            )
            .expect("control encodes"),
        ),
        Err(TransportError::CancellationInvalid)
    ));
    assert!(matches!(
        transport.receive(&[]),
        Err(TransportError::SessionClosed)
    ));
}

#[test]
fn keeps_host_policy_authoritative_over_forged_wire_context() {
    let request = format!(
        r#"{},"capabilityContext":{{"applicationId":"forged","grantedCapabilities":["diagnostics.read"]}}}}"#,
        request("platform.health", "{}")
            .strip_suffix('}')
            .expect("request ends with brace")
    );
    let mut transport = session(vec![]);
    authenticate(&mut transport);
    let response = transport
        .receive(&encode_json(&request).expect("request encodes"))
        .expect("wire accepts JSON");
    let response = decode_response(&response[0]);
    assert_eq!(
        response.as_object().expect("response object")["error"]
            .as_object()
            .expect("error object")["code"]
            .as_string(),
        Some("capability.denied")
    );
}

#[test]
fn accepts_a_granted_ui_document_replacement_after_authentication() {
    let mailbox = UiDocumentMailbox::new();
    let mut transport = TransportSession::with_ui_document_mailbox(
        HostPolicy::new(
            "test.application",
            vec![Capability::UiDocumentWrite],
            "test-host",
        )
        .expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
        mailbox.clone(),
    );
    authenticate(&mut transport);
    let response = transport
        .receive(&encode_json(&ui_document_request()).expect("request encodes"))
        .expect("authenticated request succeeds");
    let response = decode_response(&response[0]);
    assert_eq!(
        response.as_object().expect("response object")["status"].as_string(),
        Some("success")
    );
    assert_eq!(
        response.as_object().expect("response object")["result"]
            .as_object()
            .expect("result object")["revision"]
            .as_string(),
        Some("1")
    );
    let snapshot = mailbox.take().expect("accepted document is published");
    assert_eq!(snapshot.revision().value(), 1);
    assert_eq!(snapshot.document().root().id().as_str(), "root");
    assert!(mailbox.take().is_none());
}

#[test]
fn service_bundle_session_delivers_documents_to_its_host_owned_mailbox() {
    let mailbox = UiDocumentMailbox::new();
    let mut transport = TransportSession::with_session_components_and_service_bundle(
        HostPolicy::new(
            "test.application",
            vec![Capability::UiDocumentWrite],
            "test-host",
        )
        .expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
        mailbox.clone(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        HostServices::unavailable(),
    );
    authenticate(&mut transport);

    let response = transport
        .receive(&encode_json(&ui_document_request()).expect("request encodes"))
        .expect("authenticated request succeeds");
    assert_eq!(
        decode_response(&response[0])
            .as_object()
            .expect("response object")["status"]
            .as_string(),
        Some("success")
    );
    assert_eq!(
        mailbox
            .take()
            .expect("accepted document is published")
            .revision()
            .value(),
        1
    );
}

#[test]
fn grouped_session_delivers_primary_documents_through_its_group_mailbox() {
    let mailbox = UiDocumentMailbox::new();
    let group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
        mailbox.clone(),
        UiInputMailbox::new(),
    );
    let mut transport = TransportSession::with_session_window_group_and_service_bundle(
        HostPolicy::new(
            "test.application",
            vec![Capability::UiDocumentWrite],
            "test-host",
        )
        .expect("test policy is valid"),
        SessionCredentials::new(SESSION_ID, TOKEN).expect("test credentials are valid"),
        group,
        SessionCloseSignal::default(),
        HostServices::unavailable(),
    );
    authenticate(&mut transport);

    let response = transport
        .receive(&encode_json(&ui_document_request()).expect("request encodes"))
        .expect("authenticated request succeeds");
    assert_eq!(
        decode_response(&response[0])
            .as_object()
            .expect("response object")["status"]
            .as_string(),
        Some("success")
    );
    assert_eq!(
        mailbox
            .take()
            .expect("group publishes the accepted primary document")
            .revision()
            .value(),
        1
    );
}

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
