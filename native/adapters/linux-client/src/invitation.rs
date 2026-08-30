//! Exact ANLI bootstrap invitation parsing and sensitive-value handling.

use std::{fmt, io::Read};

use anodrel_client::{AuthenticationInvitation, ClientError};
use anodrel_json::JsonError;
use anodrel_protocol::{JsonValue, object};
use anodrel_transport::{SessionCredentials, authentication_message};

pub const LINUX_BOOTSTRAP_MAGIC: [u8; 4] = *b"ANLI";
pub const LINUX_BOOTSTRAP_MAJOR: u16 = 1;
pub const LINUX_BOOTSTRAP_MINOR: u16 = 0;
pub const LINUX_BOOTSTRAP_HEADER_BYTES: usize = 12;
pub const MAX_LINUX_BOOTSTRAP_PAYLOAD_BYTES: usize = 2_048;
pub const MAX_LINUX_BOOTSTRAP_FRAME_BYTES: usize =
    LINUX_BOOTSTRAP_HEADER_BYTES + MAX_LINUX_BOOTSTRAP_PAYLOAD_BYTES;

const INVITATION_KIND: &str = "linux.bootstrap.invitation";
const ENDPOINT_PREFIX: &str = "anodrel.v1.";
const ENDPOINT_HEX_CHARACTERS: usize = 64;

/// Sensitive host-created material for one Linux child connection.
///
/// Its endpoint stays private to the direct stream adapter and its token has no
/// getter. Debug output always redacts the token.
pub struct LinuxBootstrapInvitation {
    endpoint_name: String,
    session_id: String,
    token: Vec<u8>,
}

impl LinuxBootstrapInvitation {
    /// Creates one exact Linux invitation from host-owned session material.
    pub fn new(
        endpoint_name: impl Into<String>,
        session_id: impl Into<String>,
        token: impl AsRef<str>,
    ) -> Result<Self, LinuxBootstrapError> {
        let endpoint_name = endpoint_name.into();
        let session_id = session_id.into();
        let token = token.as_ref();
        if !is_valid_endpoint_name(&endpoint_name) {
            return Err(LinuxBootstrapError::InvalidInvitation);
        }
        SessionCredentials::new(session_id.clone(), token)
            .map_err(|_| LinuxBootstrapError::InvalidInvitation)?;
        Ok(Self {
            endpoint_name,
            session_id,
            token: token.as_bytes().to_vec(),
        })
    }

    /// Serializes one bounded invitation for a child-only standard-input pipe.
    pub fn encode(&self) -> Result<Vec<u8>, LinuxBootstrapError> {
        let token =
            std::str::from_utf8(&self.token).map_err(|_| LinuxBootstrapError::InvalidInvitation)?;
        let payload = object([
            ("kind", JsonValue::String(INVITATION_KIND.to_owned())),
            (
                "endpointName",
                JsonValue::String(self.endpoint_name.clone()),
            ),
            (
                "protocolVersion",
                object([
                    (
                        "major",
                        JsonValue::Number(LINUX_BOOTSTRAP_MAJOR.to_string()),
                    ),
                    (
                        "minor",
                        JsonValue::Number(LINUX_BOOTSTRAP_MINOR.to_string()),
                    ),
                ]),
            ),
            ("sessionId", JsonValue::String(self.session_id.clone())),
            ("token", JsonValue::String(token.to_owned())),
        ])
        .to_json()
        .into_bytes();
        if payload.len() > MAX_LINUX_BOOTSTRAP_PAYLOAD_BYTES {
            return Err(LinuxBootstrapError::OversizedPayload);
        }
        let length =
            u32::try_from(payload.len()).map_err(|_| LinuxBootstrapError::OversizedPayload)?;
        let mut frame = Vec::with_capacity(LINUX_BOOTSTRAP_HEADER_BYTES + payload.len());
        frame.extend_from_slice(&LINUX_BOOTSTRAP_MAGIC);
        frame.extend_from_slice(&LINUX_BOOTSTRAP_MAJOR.to_le_bytes());
        frame.extend_from_slice(&LINUX_BOOTSTRAP_MINOR.to_le_bytes());
        frame.extend_from_slice(&length.to_le_bytes());
        frame.extend_from_slice(&payload);
        Ok(frame)
    }

    /// Parses one complete exact ANLI frame.
    pub fn decode(frame: &[u8]) -> Result<Self, LinuxBootstrapError> {
        if frame.len() < LINUX_BOOTSTRAP_HEADER_BYTES {
            return Err(LinuxBootstrapError::TruncatedFrame);
        }
        if frame[..4] != LINUX_BOOTSTRAP_MAGIC {
            return Err(LinuxBootstrapError::InvalidMagic);
        }
        let major = u16::from_le_bytes([frame[4], frame[5]]);
        let minor = u16::from_le_bytes([frame[6], frame[7]]);
        if major != LINUX_BOOTSTRAP_MAJOR || minor != LINUX_BOOTSTRAP_MINOR {
            return Err(LinuxBootstrapError::UnsupportedVersion { major, minor });
        }
        let payload_length =
            u32::from_le_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
        if payload_length > MAX_LINUX_BOOTSTRAP_PAYLOAD_BYTES {
            return Err(LinuxBootstrapError::OversizedPayload);
        }
        if frame.len() != LINUX_BOOTSTRAP_HEADER_BYTES + payload_length {
            return Err(LinuxBootstrapError::TruncatedFrame);
        }
        let payload = std::str::from_utf8(&frame[LINUX_BOOTSTRAP_HEADER_BYTES..])
            .map_err(|_| LinuxBootstrapError::InvalidUtf8)?;
        parse_payload(payload)
    }

    /// Reads one invitation through standard-input end-of-file.
    pub fn read_from(input: &mut impl Read) -> Result<Self, LinuxBootstrapError> {
        let mut frame = Vec::with_capacity(MAX_LINUX_BOOTSTRAP_FRAME_BYTES);
        let read_result = input
            .take((MAX_LINUX_BOOTSTRAP_FRAME_BYTES + 1) as u64)
            .read_to_end(&mut frame);
        if let Err(error) = read_result {
            frame.fill(0);
            return Err(LinuxBootstrapError::Read(error));
        }
        if frame.len() > MAX_LINUX_BOOTSTRAP_FRAME_BYTES {
            frame.fill(0);
            return Err(LinuxBootstrapError::OversizedPayload);
        }
        let invitation = Self::decode(&frame);
        frame.fill(0);
        invitation
    }

    pub(crate) fn endpoint_name(&self) -> &str {
        &self.endpoint_name
    }

    fn authentication_message(&self) -> Result<String, LinuxBootstrapError> {
        let token =
            std::str::from_utf8(&self.token).map_err(|_| LinuxBootstrapError::InvalidInvitation)?;
        authentication_message(&self.session_id, token)
            .map_err(|_| LinuxBootstrapError::InvalidInvitation)
    }
}

impl AuthenticationInvitation for LinuxBootstrapInvitation {
    fn authentication_message(&self) -> Result<String, ClientError> {
        self.authentication_message()
            .map_err(|_| ClientError::BootstrapUnreadable)
    }
}

impl fmt::Debug for LinuxBootstrapInvitation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxBootstrapInvitation")
            .field("endpoint_name", &self.endpoint_name)
            .field("session_id", &self.session_id)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl Drop for LinuxBootstrapInvitation {
    fn drop(&mut self) {
        self.token.fill(0);
    }
}

/// Closed outcomes for malformed or unreadable Linux bootstrap material.
#[derive(Debug)]
pub enum LinuxBootstrapError {
    InvalidInvitation,
    InvalidMagic,
    UnsupportedVersion { major: u16, minor: u16 },
    OversizedPayload,
    TruncatedFrame,
    InvalidUtf8,
    InvalidJson(JsonError),
    Read(std::io::Error),
}

impl fmt::Display for LinuxBootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInvitation => write!(formatter, "Linux bootstrap invitation is invalid"),
            Self::InvalidMagic => write!(formatter, "Linux bootstrap frame magic is invalid"),
            Self::UnsupportedVersion { major, minor } => {
                write!(
                    formatter,
                    "Linux bootstrap frame version {major}.{minor} is unsupported"
                )
            }
            Self::OversizedPayload => {
                write!(formatter, "Linux bootstrap payload exceeds its limit")
            }
            Self::TruncatedFrame => {
                write!(
                    formatter,
                    "Linux bootstrap frame is truncated or has trailing data"
                )
            }
            Self::InvalidUtf8 => write!(formatter, "Linux bootstrap payload is not valid UTF-8"),
            Self::InvalidJson(error) => {
                write!(
                    formatter,
                    "Linux bootstrap payload is invalid JSON: {error}"
                )
            }
            Self::Read(error) => write!(
                formatter,
                "Linux bootstrap stream could not be read: {error}"
            ),
        }
    }
}

impl std::error::Error for LinuxBootstrapError {}

fn parse_payload(payload: &str) -> Result<LinuxBootstrapInvitation, LinuxBootstrapError> {
    let value = JsonValue::parse(payload).map_err(LinuxBootstrapError::InvalidJson)?;
    let fields = value
        .as_object()
        .ok_or(LinuxBootstrapError::InvalidInvitation)?;
    if fields.len() != 5
        || required_string(fields, "kind")? != INVITATION_KIND
        || !is_valid_version(fields)?
    {
        return Err(LinuxBootstrapError::InvalidInvitation);
    }
    LinuxBootstrapInvitation::new(
        required_string(fields, "endpointName")?,
        required_string(fields, "sessionId")?,
        required_string(fields, "token")?,
    )
}

fn is_valid_version(
    fields: &std::collections::BTreeMap<String, JsonValue>,
) -> Result<bool, LinuxBootstrapError> {
    let version = fields
        .get("protocolVersion")
        .and_then(JsonValue::as_object)
        .ok_or(LinuxBootstrapError::InvalidInvitation)?;
    Ok(version.len() == 2
        && version.get("major").and_then(JsonValue::as_u16) == Some(LINUX_BOOTSTRAP_MAJOR)
        && version.get("minor").and_then(JsonValue::as_u16) == Some(LINUX_BOOTSTRAP_MINOR))
}

fn required_string<'a>(
    fields: &'a std::collections::BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, LinuxBootstrapError> {
    fields
        .get(name)
        .and_then(JsonValue::as_string)
        .ok_or(LinuxBootstrapError::InvalidInvitation)
}

fn is_valid_endpoint_name(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix(ENDPOINT_PREFIX) else {
        return false;
    };
    suffix.len() == ENDPOINT_HEX_CHARACTERS
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{
        LINUX_BOOTSTRAP_MAGIC, LINUX_BOOTSTRAP_MAJOR, LINUX_BOOTSTRAP_MINOR, LinuxBootstrapError,
        LinuxBootstrapInvitation,
    };
    use anodrel_client::AuthenticationInvitation;

    const ENDPOINT: &str =
        "anodrel.v1.0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const SESSION_ID: &str = "linux-client-test";
    const TOKEN: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn invitation() -> LinuxBootstrapInvitation {
        LinuxBootstrapInvitation::new(ENDPOINT, SESSION_ID, TOKEN).expect("invitation is valid")
    }

    #[test]
    fn round_trips_an_exact_linux_invitation() {
        let decoded = LinuxBootstrapInvitation::decode(&invitation().encode().expect("encodes"))
            .expect("decodes");
        assert!(format!("{decoded:?}").contains(ENDPOINT));
    }

    #[test]
    fn does_not_accept_the_windows_magic_or_extra_payload_fields() {
        let mut wrong_magic = invitation().encode().expect("encodes");
        wrong_magic[..4].copy_from_slice(b"ANBI");
        assert!(matches!(
            LinuxBootstrapInvitation::decode(&wrong_magic),
            Err(LinuxBootstrapError::InvalidMagic)
        ));

        let payload = format!(
            "{{\"endpointName\":\"{ENDPOINT}\",\"extra\":true,\"kind\":\"linux.bootstrap.invitation\",\"protocolVersion\":{{\"major\":1,\"minor\":0}},\"sessionId\":\"{SESSION_ID}\",\"token\":\"{TOKEN}\"}}"
        );
        let mut frame = Vec::new();
        frame.extend_from_slice(&LINUX_BOOTSTRAP_MAGIC);
        frame.extend_from_slice(&LINUX_BOOTSTRAP_MAJOR.to_le_bytes());
        frame.extend_from_slice(&LINUX_BOOTSTRAP_MINOR.to_le_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload.as_bytes());
        assert!(matches!(
            LinuxBootstrapInvitation::decode(&frame),
            Err(LinuxBootstrapError::InvalidInvitation)
        ));
    }

    #[test]
    fn redacts_the_token_and_builds_the_authentication_control() {
        let invitation = invitation();
        let debug = format!("{invitation:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains(TOKEN));
        assert!(
            AuthenticationInvitation::authentication_message(&invitation)
                .expect("authentication is available")
                .contains(SESSION_ID)
        );
    }
}
