#![forbid(unsafe_code)]

//! The portable child half of one authenticated Anodrel conversation.
//!
//! This crate owns no operating-system handle or endpoint. A platform adapter
//! opens the exact private stream named by a [`BootstrapInvitation`], then this
//! module emits authentication first and exchanges one ordered request at a
//! time. See `docs/NATIVE_CLIENT.md`.

mod interactive_poll;

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    io::{Read, Write},
};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_json::JsonValue;
use anodrel_wire::{FrameDecoder, encode_json};

pub use interactive_poll::InteractivePollSchedule;

/// The fixed stream read buffer for one client receive operation.
///
/// It matches the first Windows adapter's own pipe buffer. The wire decoder,
/// rather than this buffer size, owns the protocol's total receive limit.
pub const READ_BUFFER_BYTES: usize = 4 * 1024;

/// A version for one protocol request.
///
/// The wire version is deliberately not carried here; `anodrel-wire` owns that
/// independent framing contract. A child chooses the documented protocol minor
/// needed by its fixed behaviour and the host still decides whether it accepts
/// it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolVersion {
    minor: u16,
}

impl ProtocolVersion {
    /// Creates a Protocol 1 request version with the supplied documented minor.
    #[must_use]
    pub const fn v1(minor: u16) -> Self {
        Self { minor }
    }

    /// Returns the Protocol 1 minor carried in a request envelope.
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }
}

/// Closed outcomes from the native child conversation.
///
/// This value intentionally keeps no I/O error, endpoint, bootstrap material,
/// raw response, or host failure payload. A child may map it to a safe fixed
/// exit status, but it must not print sensitive transport detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientError {
    /// Standard input did not contain one valid bootstrap invitation.
    BootstrapUnreadable,
    /// The invited byte stream could not complete a read or write.
    StreamUnavailable,
    /// The peer ended its stream before the expected frame arrived.
    StreamEnded,
    /// The bounded Anodrel wire frame was malformed or unsupported.
    FrameInvalid,
    /// Authentication did not receive the exact host acknowledgement.
    AuthenticationRejected,
    /// A response was not a valid response envelope for the sent request.
    ResponseInvalid,
    /// A valid host response reported failure rather than a result.
    RequestRejected,
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::BootstrapUnreadable => "the bootstrap invitation was unavailable",
            Self::StreamUnavailable => "the invited stream was unavailable",
            Self::StreamEnded => "the invited stream ended",
            Self::FrameInvalid => "the invited stream carried an invalid frame",
            Self::AuthenticationRejected => "the host rejected authentication",
            Self::ResponseInvalid => "the host response was invalid",
            Self::RequestRejected => "the host rejected the request",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ClientError {}

/// One authenticated, ordered Anodrel client conversation.
///
/// Construction consumes the invitation after building the authentication
/// message, so the child cannot retain its bootstrap token after the handshake.
/// A `Client` has no method for unauthenticated public requests.
pub struct Client<Stream> {
    stream: Stream,
    decoder: FrameDecoder,
    pending: VecDeque<String>,
}

impl<Stream> Client<Stream>
where
    Stream: Read + Write,
{
    /// Reads one private invitation from the supplied end-of-file-delimited
    /// bootstrap stream.
    ///
    /// Every bootstrap failure maps to one safe category, so a child cannot
    /// accidentally propagate a token, malformed payload, or system I/O detail
    /// to its caller.
    pub fn read_invitation(input: &mut impl Read) -> Result<BootstrapInvitation, ClientError> {
        BootstrapInvitation::read_from(input).map_err(|_| ClientError::BootstrapUnreadable)
    }

    /// Authenticates an already-open invited stream.
    ///
    /// The platform adapter must have opened only `invitation.pipe_name()` and
    /// must not use the value to construct or discover another endpoint.
    pub fn authenticate(
        stream: Stream,
        invitation: BootstrapInvitation,
    ) -> Result<Self, ClientError> {
        let authentication = invitation
            .authentication_message()
            .map_err(|_| ClientError::BootstrapUnreadable)?;
        let mut client = Self {
            stream,
            decoder: FrameDecoder::new(),
            pending: VecDeque::new(),
        };
        client.send(&authentication)?;
        // `invitation` is consumed by this function and drops immediately after
        // this call, zeroing its token before any application request is sent.
        let acknowledgement = client.receive()?;
        if object_string(&acknowledgement, "kind") != Some("session.authenticated") {
            return Err(ClientError::AuthenticationRejected);
        }
        Ok(client)
    }

    /// Sends one documented request and returns only its successful result.
    ///
    /// The payload is structured `JsonValue`, never interpolated text, so a
    /// caller-provided string cannot escape its JSON field. The client allows
    /// only one request awaiting a response; that makes matching exact and
    /// leaves no background receiver or unbounded pending-request map.
    pub fn request(
        &mut self,
        version: ProtocolVersion,
        request_id: &str,
        operation: &str,
        payload: JsonValue,
    ) -> Result<JsonValue, ClientError> {
        let request = JsonValue::Object(BTreeMap::from([
            ("kind".to_owned(), JsonValue::String("request".to_owned())),
            (
                "operation".to_owned(),
                JsonValue::String(operation.to_owned()),
            ),
            ("payload".to_owned(), payload),
            (
                "protocolVersion".to_owned(),
                JsonValue::Object(BTreeMap::from([
                    ("major".to_owned(), JsonValue::Number("1".to_owned())),
                    (
                        "minor".to_owned(),
                        JsonValue::Number(version.minor().to_string()),
                    ),
                ])),
            ),
            (
                "requestId".to_owned(),
                JsonValue::String(request_id.to_owned()),
            ),
        ]))
        .to_json();
        self.send(&request)?;
        let response = self.receive()?;
        let Some(fields) = response.as_object() else {
            return Err(ClientError::ResponseInvalid);
        };
        if fields.get("kind").and_then(JsonValue::as_string) != Some("response")
            || fields.get("requestId").and_then(JsonValue::as_string) != Some(request_id)
        {
            return Err(ClientError::ResponseInvalid);
        }
        match fields.get("status").and_then(JsonValue::as_string) {
            Some("success") => fields
                .get("result")
                .cloned()
                .ok_or(ClientError::ResponseInvalid),
            Some("failure") => Err(ClientError::RequestRejected),
            _ => Err(ClientError::ResponseInvalid),
        }
    }

    fn send(&mut self, message: &str) -> Result<(), ClientError> {
        let frame = encode_json(message).map_err(|_| ClientError::FrameInvalid)?;
        self.stream
            .write_all(&frame)
            .map_err(|_| ClientError::StreamUnavailable)?;
        self.stream
            .flush()
            .map_err(|_| ClientError::StreamUnavailable)
    }

    fn receive(&mut self) -> Result<JsonValue, ClientError> {
        let mut buffer = [0_u8; READ_BUFFER_BYTES];
        loop {
            if let Some(message) = self.pending.pop_front() {
                return JsonValue::parse(&message).map_err(|_| ClientError::ResponseInvalid);
            }
            let read = self
                .stream
                .read(&mut buffer)
                .map_err(|_| ClientError::StreamUnavailable)?;
            if read == 0 {
                return Err(ClientError::StreamEnded);
            }
            let messages = self
                .decoder
                .push(&buffer[..read])
                .map_err(|_| ClientError::FrameInvalid)?;
            self.pending.extend(messages);
        }
    }
}

impl<Stream> fmt::Debug for Client<Stream> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The stream can encapsulate a host-private handle, so never delegate
        // its debug representation to an application or diagnostic surface.
        formatter
            .debug_struct("Client")
            .field("pending_frames", &self.pending.len())
            .finish_non_exhaustive()
    }
}

fn object_string<'a>(value: &'a JsonValue, name: &str) -> Option<&'a str> {
    value.as_object()?.get(name)?.as_string()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        io::{self, Read, Write},
    };

    use anodrel_bootstrap::BootstrapInvitation;
    use anodrel_json::JsonValue;
    use anodrel_wire::{FrameDecoder, encode_json};

    use super::{Client, ClientError, ProtocolVersion, object_string};

    const PIPE_NAME: &str = r"\\.\pipe\anodrel.v1.client-test";
    const SESSION_ID: &str = "client-test-session";
    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[derive(Debug, Default)]
    struct TestStream {
        reads: VecDeque<Vec<u8>>,
        written: Vec<u8>,
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
    fn preserves_a_coalesced_later_response_for_the_next_request() {
        let mut coalesced = frame(
            r#"{"kind":"response","requestId":"one","status":"success","result":{"status":"first"}}"#,
        );
        coalesced.extend(frame(
            r#"{"kind":"response","requestId":"two","status":"success","result":{"status":"second"}}"#,
        ));
        let stream =
            TestStream::with_reads([frame(r#"{"kind":"session.authenticated"}"#), coalesced]);
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
        let mut client =
            Client::authenticate(stream, invitation()).expect("fragments authenticate");
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
}
