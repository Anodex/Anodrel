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

/// Host-created secret material that can build exactly one authentication
/// message for an already-open invited stream.
///
/// Platform invitation codecs keep endpoint validation and secret storage on
/// their own side of this boundary. This portable client never selects or opens
/// an operating-system endpoint.
pub trait AuthenticationInvitation {
    /// Produces the first private session control without exposing its token.
    fn authentication_message(&self) -> Result<String, ClientError>;
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
    pub fn authenticate<Invitation>(
        stream: Stream,
        invitation: Invitation,
    ) -> Result<Self, ClientError>
    where
        Invitation: AuthenticationInvitation,
    {
        let authentication = invitation.authentication_message()?;
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

impl AuthenticationInvitation for BootstrapInvitation {
    fn authentication_message(&self) -> Result<String, ClientError> {
        BootstrapInvitation::authentication_message(self)
            .map_err(|_| ClientError::BootstrapUnreadable)
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
mod tests;
