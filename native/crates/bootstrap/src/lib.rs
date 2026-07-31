#![forbid(unsafe_code)]

//! A bounded, one-record private invitation codec for launched applications.
//!
//! The bootstrap frame is not part of the public request protocol. It carries
//! pipe credentials only from a host to its newly launched child, then the
//! child must use the normal authenticated transport.

use std::{fmt, io::Read};

use anodrel_json::JsonError;
use anodrel_protocol::{JsonValue, object};
use anodrel_transport::SessionCredentials;

pub const BOOTSTRAP_MAGIC: [u8; 4] = *b"ANBI";
pub const BOOTSTRAP_MAJOR: u16 = 1;
pub const BOOTSTRAP_MINOR: u16 = 0;
pub const BOOTSTRAP_HEADER_BYTES: usize = 12;
pub const MAX_BOOTSTRAP_PAYLOAD_BYTES: usize = 2_048;
pub const MAX_BOOTSTRAP_FRAME_BYTES: usize = BOOTSTRAP_HEADER_BYTES + MAX_BOOTSTRAP_PAYLOAD_BYTES;
pub const MAX_PIPE_NAME_BYTES: usize = 512;

const INVITATION_KIND: &str = "bootstrap.invitation";
const PIPE_PREFIX: &str = r"\\.\pipe\anodrel.v1.";

/// Secret connection material delivered once to a launched child process.
/// `Debug` and public accessors never reveal the token.
pub struct BootstrapInvitation {
    pipe_name: String,
    session_id: String,
    token: Vec<u8>,
}

impl BootstrapInvitation {
    pub fn new(
        pipe_name: impl Into<String>,
        session_id: impl Into<String>,
        token: impl AsRef<str>,
    ) -> Result<Self, BootstrapError> {
        let pipe_name = pipe_name.into();
        let session_id = session_id.into();
        let token = token.as_ref();
        if !is_valid_pipe_name(&pipe_name) {
            return Err(BootstrapError::InvalidInvitation);
        }
        SessionCredentials::new(session_id.clone(), token)
            .map_err(|_| BootstrapError::InvalidInvitation)?;
        Ok(Self {
            pipe_name,
            session_id,
            token: token.as_bytes().to_vec(),
        })
    }

    pub fn pipe_name(&self) -> &str {
        &self.pipe_name
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Builds the complete one-use frame. Callers must write it exactly once
    /// to a child-only bootstrap channel and then close that channel.
    pub fn encode(&self) -> Result<Vec<u8>, BootstrapError> {
        let token =
            std::str::from_utf8(&self.token).map_err(|_| BootstrapError::InvalidInvitation)?;
        let payload = object([
            ("kind", JsonValue::String(INVITATION_KIND.to_owned())),
            ("pipeName", JsonValue::String(self.pipe_name.clone())),
            (
                "protocolVersion",
                object([
                    ("major", JsonValue::Number(BOOTSTRAP_MAJOR.to_string())),
                    ("minor", JsonValue::Number(BOOTSTRAP_MINOR.to_string())),
                ]),
            ),
            ("sessionId", JsonValue::String(self.session_id.clone())),
            ("token", JsonValue::String(token.to_owned())),
        ])
        .to_json();
        let payload = payload.into_bytes();
        if payload.len() > MAX_BOOTSTRAP_PAYLOAD_BYTES {
            return Err(BootstrapError::OversizedPayload);
        }
        let payload_length =
            u32::try_from(payload.len()).map_err(|_| BootstrapError::OversizedPayload)?;
        let mut frame = Vec::with_capacity(BOOTSTRAP_HEADER_BYTES + payload.len());
        frame.extend_from_slice(&BOOTSTRAP_MAGIC);
        frame.extend_from_slice(&BOOTSTRAP_MAJOR.to_le_bytes());
        frame.extend_from_slice(&BOOTSTRAP_MINOR.to_le_bytes());
        frame.extend_from_slice(&payload_length.to_le_bytes());
        frame.extend_from_slice(&payload);
        Ok(frame)
    }

    /// Parses exactly one complete bootstrap frame. The caller must supply the
    /// full byte stream after its bootstrap handle reaches end-of-file.
    pub fn decode(frame: &[u8]) -> Result<Self, BootstrapError> {
        if frame.len() < BOOTSTRAP_HEADER_BYTES {
            return Err(BootstrapError::TruncatedFrame);
        }
        if frame[..4] != BOOTSTRAP_MAGIC {
            return Err(BootstrapError::InvalidMagic);
        }
        let major = u16::from_le_bytes([frame[4], frame[5]]);
        let minor = u16::from_le_bytes([frame[6], frame[7]]);
        if major != BOOTSTRAP_MAJOR || minor != BOOTSTRAP_MINOR {
            return Err(BootstrapError::UnsupportedVersion { major, minor });
        }
        let payload_length =
            u32::from_le_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
        if payload_length > MAX_BOOTSTRAP_PAYLOAD_BYTES {
            return Err(BootstrapError::OversizedPayload);
        }
        if frame.len() != BOOTSTRAP_HEADER_BYTES + payload_length {
            return Err(BootstrapError::TruncatedFrame);
        }
        let payload = std::str::from_utf8(&frame[BOOTSTRAP_HEADER_BYTES..])
            .map_err(|_| BootstrapError::InvalidUtf8)?;
        parse_payload(payload)
    }

    /// Reads one complete invitation through end-of-file without allocating
    /// beyond the documented frame limit.
    pub fn read_from(input: &mut impl Read) -> Result<Self, BootstrapError> {
        let mut frame = Vec::with_capacity(MAX_BOOTSTRAP_FRAME_BYTES);
        let read_result = input
            .take((MAX_BOOTSTRAP_FRAME_BYTES + 1) as u64)
            .read_to_end(&mut frame);
        if let Err(error) = read_result {
            frame.fill(0);
            return Err(BootstrapError::Read(error));
        }
        if frame.len() > MAX_BOOTSTRAP_FRAME_BYTES {
            frame.fill(0);
            return Err(BootstrapError::OversizedPayload);
        }
        let invitation = Self::decode(&frame);
        frame.fill(0);
        invitation
    }
}

impl fmt::Debug for BootstrapInvitation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BootstrapInvitation")
            .field("pipe_name", &self.pipe_name)
            .field("session_id", &self.session_id)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl Drop for BootstrapInvitation {
    fn drop(&mut self) {
        self.token.fill(0);
    }
}

#[derive(Debug)]
pub enum BootstrapError {
    InvalidInvitation,
    InvalidMagic,
    UnsupportedVersion { major: u16, minor: u16 },
    OversizedPayload,
    TruncatedFrame,
    InvalidUtf8,
    InvalidJson(JsonError),
    Read(std::io::Error),
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInvitation => write!(formatter, "bootstrap invitation is invalid"),
            Self::InvalidMagic => write!(formatter, "bootstrap frame magic is invalid"),
            Self::UnsupportedVersion { major, minor } => {
                write!(
                    formatter,
                    "bootstrap frame version {major}.{minor} is unsupported"
                )
            }
            Self::OversizedPayload => write!(formatter, "bootstrap payload exceeds its limit"),
            Self::TruncatedFrame => write!(
                formatter,
                "bootstrap frame is truncated or has trailing data"
            ),
            Self::InvalidUtf8 => write!(formatter, "bootstrap payload is not valid UTF-8"),
            Self::InvalidJson(error) => {
                write!(formatter, "bootstrap payload is invalid JSON: {error}")
            }
            Self::Read(error) => write!(formatter, "bootstrap stream could not be read: {error}"),
        }
    }
}

impl std::error::Error for BootstrapError {}

fn parse_payload(payload: &str) -> Result<BootstrapInvitation, BootstrapError> {
    let value = JsonValue::parse(payload).map_err(BootstrapError::InvalidJson)?;
    let fields = value.as_object().ok_or(BootstrapError::InvalidInvitation)?;
    if fields.len() != 5 {
        return Err(BootstrapError::InvalidInvitation);
    }
    let kind = required_string(fields, "kind")?;
    if kind != INVITATION_KIND {
        return Err(BootstrapError::InvalidInvitation);
    }
    let version = fields
        .get("protocolVersion")
        .and_then(JsonValue::as_object)
        .ok_or(BootstrapError::InvalidInvitation)?;
    if version.len() != 2
        || version.get("major").and_then(JsonValue::as_u16) != Some(BOOTSTRAP_MAJOR)
        || version.get("minor").and_then(JsonValue::as_u16) != Some(BOOTSTRAP_MINOR)
    {
        return Err(BootstrapError::InvalidInvitation);
    }
    BootstrapInvitation::new(
        required_string(fields, "pipeName")?,
        required_string(fields, "sessionId")?,
        required_string(fields, "token")?,
    )
}

fn required_string<'a>(
    fields: &'a std::collections::BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, BootstrapError> {
    fields
        .get(name)
        .and_then(JsonValue::as_string)
        .ok_or(BootstrapError::InvalidInvitation)
}

fn is_valid_pipe_name(value: &str) -> bool {
    value.starts_with(PIPE_PREFIX)
        && value.len() <= MAX_PIPE_NAME_BYTES
        && value.is_ascii()
        && value.len() > PIPE_PREFIX.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIPE_NAME: &str = r"\\.\pipe\anodrel.v1.test";
    const SESSION_ID: &str = "test-session";
    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn invitation() -> BootstrapInvitation {
        BootstrapInvitation::new(PIPE_NAME, SESSION_ID, TOKEN).expect("invitation is valid")
    }

    #[test]
    fn round_trips_a_strict_invitation() {
        let decoded =
            BootstrapInvitation::decode(&invitation().encode().expect("encodes")).expect("decodes");
        assert_eq!(decoded.pipe_name(), PIPE_NAME);
        assert_eq!(decoded.session_id(), SESSION_ID);
    }

    #[test]
    fn redacts_the_token_in_debug_output() {
        let debug = format!("{:?}", invitation());
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains(TOKEN));
    }

    #[test]
    fn rejects_bad_version_and_trailing_data() {
        let mut bad_version = invitation().encode().expect("encodes");
        bad_version[4] = 2;
        assert!(matches!(
            BootstrapInvitation::decode(&bad_version),
            Err(BootstrapError::UnsupportedVersion { .. })
        ));
        let mut trailing = invitation().encode().expect("encodes");
        trailing.push(0);
        assert!(matches!(
            BootstrapInvitation::decode(&trailing),
            Err(BootstrapError::TruncatedFrame)
        ));
    }

    #[test]
    fn rejects_wrong_or_unknown_payload_fields() {
        let payload = r#"{"kind":"bootstrap.invitation","pipeName":"\\\\.\\pipe\\anodrel.v1.test","protocolVersion":{"major":1,"minor":0},"sessionId":"test-session","token":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","extra":true}"#;
        let mut frame = Vec::new();
        frame.extend_from_slice(&BOOTSTRAP_MAGIC);
        frame.extend_from_slice(&BOOTSTRAP_MAJOR.to_le_bytes());
        frame.extend_from_slice(&BOOTSTRAP_MINOR.to_le_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload.as_bytes());
        assert!(matches!(
            BootstrapInvitation::decode(&frame),
            Err(BootstrapError::InvalidInvitation)
        ));
    }

    #[test]
    fn bounds_stream_reads_and_uses_end_of_file() {
        let valid_bytes = invitation().encode().expect("encodes");
        let mut valid = valid_bytes.as_slice();
        assert_eq!(
            BootstrapInvitation::read_from(&mut valid)
                .expect("reads invitation")
                .session_id(),
            SESSION_ID
        );
        let too_large_bytes = vec![0_u8; MAX_BOOTSTRAP_FRAME_BYTES + 1];
        let mut too_large = too_large_bytes.as_slice();
        assert!(matches!(
            BootstrapInvitation::read_from(&mut too_large),
            Err(BootstrapError::OversizedPayload)
        ));
    }
}
