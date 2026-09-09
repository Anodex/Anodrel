//! Contract tests for the private authenticated client conversation.

use std::{
    collections::VecDeque,
    io::{self, Read, Write},
};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_json::JsonValue;
use anodrel_wire::{FrameDecoder, encode_json};

use super::{AuthenticationInvitation, Client, ClientError, ProtocolVersion, object_string};

const PIPE_NAME: &str = r"\\.\pipe\anodrel.v1.client-test";
const SESSION_ID: &str = "client-test-session";
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[derive(Debug, Default)]
struct TestStream {
    reads: VecDeque<Vec<u8>>,
    written: Vec<u8>,
}

struct AlternateInvitation;

impl AuthenticationInvitation for AlternateInvitation {
    fn authentication_message(&self) -> Result<String, ClientError> {
        Ok(format!(
            r#"{{"kind":"session.authenticate","sessionId":"{SESSION_ID}","token":"{TOKEN}"}}"#
        ))
    }
}

impl TestStream {
    fn with_reads(reads: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            reads: reads.into_iter().collect(),
            written: Vec::new(),
        }
    }

    fn messages(&self) -> Vec<String> {
        let mut decoder = FrameDecoder::new();
        decoder
            .push(&self.written)
            .expect("client wrote valid frames")
    }
}

impl Read for TestStream {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let Some(next) = self.reads.pop_front() else {
            return Ok(0);
        };
        assert!(next.len() <= output.len(), "test chunk must fit the buffer");
        let length = next.len();
        output[..length].copy_from_slice(&next);
        Ok(length)
    }
}

impl Write for TestStream {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        self.written.extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn invitation() -> BootstrapInvitation {
    BootstrapInvitation::new(PIPE_NAME, SESSION_ID, TOKEN).expect("invitation is valid")
}

fn frame(message: &str) -> Vec<u8> {
    encode_json(message).expect("test response encodes")
}

#[test]
fn authenticates_before_it_can_send_a_public_request() {
    let stream = TestStream::with_reads([
        frame(r#"{"kind":"session.authenticated"}"#),
        frame(
            r#"{"kind":"response","requestId":"health","status":"success","result":{"status":"ready"}}"#,
        ),
    ]);
    let mut client = Client::authenticate(stream, invitation()).expect("host authenticates");
    let result = client
        .request(
            ProtocolVersion::v1(0),
            "health",
            "platform.health",
            JsonValue::Object(Default::default()),
        )
        .expect("host returns health");
    assert_eq!(object_string(&result, "status"), Some("ready"));

    let messages = client.stream.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(
        object_string(
            &JsonValue::parse(&messages[0]).expect("authentication is JSON"),
            "kind"
        ),
        Some("session.authenticate")
    );
    let request = JsonValue::parse(&messages[1]).expect("request is JSON");
    assert_eq!(
        object_string(&request, "operation"),
        Some("platform.health")
    );
}

#[test]
fn authenticates_through_a_platform_invitation_implementation() {
    let stream = TestStream::with_reads([frame(r#"{"kind":"session.authenticated"}"#)]);
    let client = Client::authenticate(stream, AlternateInvitation)
        .expect("a platform invitation supplies authentication");
    assert_eq!(client.stream.messages().len(), 1);
}

#[test]
fn preserves_a_coalesced_later_response_for_the_next_request() {
    let mut coalesced = frame(
        r#"{"kind":"response","requestId":"one","status":"success","result":{"status":"first"}}"#,
    );
    coalesced.extend(frame(
        r#"{"kind":"response","requestId":"two","status":"success","result":{"status":"second"}}"#,
    ));
    let stream = TestStream::with_reads([frame(r#"{"kind":"session.authenticated"}"#), coalesced]);
    let mut client = Client::authenticate(stream, invitation()).expect("host authenticates");

    let first = client
        .request(
            ProtocolVersion::v1(0),
            "one",
            "platform.health",
            JsonValue::Object(Default::default()),
        )
        .expect("first result returns");
    let second = client
        .request(
            ProtocolVersion::v1(0),
            "two",
            "platform.health",
            JsonValue::Object(Default::default()),
        )
        .expect("queued result returns");
    assert_eq!(object_string(&first, "status"), Some("first"));
    assert_eq!(object_string(&second, "status"), Some("second"));
}

#[test]
fn accepts_a_fragmented_authentication_acknowledgement() {
    let acknowledgement = frame(r#"{"kind":"session.authenticated"}"#);
    let stream = TestStream::with_reads([
        acknowledgement[..7].to_vec(),
        acknowledgement[7..].to_vec(),
        frame(r#"{"kind":"response","requestId":"health","status":"success","result":{}}"#),
    ]);
    let mut client = Client::authenticate(stream, invitation()).expect("fragments authenticate");
    assert!(
        client
            .request(
                ProtocolVersion::v1(0),
                "health",
                "platform.health",
                JsonValue::Object(Default::default()),
            )
            .is_ok()
    );
}

#[test]
fn rejected_response_is_a_closed_category_without_payload_retention() {
    let stream = TestStream::with_reads([
        frame(r#"{"kind":"session.authenticated"}"#),
        frame(
            r#"{"kind":"response","requestId":"denied","status":"failure","error":{"message":"private endpoint detail"}}"#,
        ),
    ]);
    let mut client = Client::authenticate(stream, invitation()).expect("host authenticates");
    assert_eq!(
        client.request(
            ProtocolVersion::v1(0),
            "denied",
            "platform.health",
            JsonValue::Object(Default::default()),
        ),
        Err(ClientError::RequestRejected)
    );
    assert!(!format!("{:?}", ClientError::RequestRejected).contains("private"));
}

#[test]
fn response_for_another_request_is_invalid() {
    let stream = TestStream::with_reads([
        frame(r#"{"kind":"session.authenticated"}"#),
        frame(r#"{"kind":"response","requestId":"other","status":"success","result":{}}"#),
    ]);
    let mut client = Client::authenticate(stream, invitation()).expect("host authenticates");
    assert_eq!(
        client.request(
            ProtocolVersion::v1(0),
            "expected",
            "platform.health",
            JsonValue::Object(Default::default()),
        ),
        Err(ClientError::ResponseInvalid)
    );
}

#[test]
fn bootstrap_read_failures_are_collapsed_before_they_leave_the_client() {
    let mut invalid = &b"not an invitation"[..];
    assert!(matches!(
        Client::<TestStream>::read_invitation(&mut invalid),
        Err(ClientError::BootstrapUnreadable)
    ));
}
