#![forbid(unsafe_code)]

//! A policy-bound, authenticated, byte-stream-independent native host session.
//!
//! OS adapters own invitation delivery and blocking I/O. This module owns the
//! bounded transition from framed input to complete core responses and refuses
//! every public protocol request until host-created credentials are verified.

mod session;

pub use session::TransportSession;

use std::{collections::BTreeSet, fmt};

use anodrel_clipboard::{ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText};
use anodrel_core::{CoreHost, HostPolicy, HostServices, SessionCloseSignal};
use anodrel_credentials::{CredentialName, CredentialService, CredentialServiceError, Secret};
use anodrel_diagnostics::DiagnosticsService;
use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
use anodrel_file_access::{FileSelectionService, FileTextService};
use anodrel_file_dialog::{
    FileDialogFilter, FileDialogSelection, FileDialogService, FileDialogServiceError,
};
use anodrel_protocol::{CancellationEnvelope, JsonValue, RequestEnvelope, object};
use anodrel_storage::StorageService;
pub use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_window::WindowTitleProposal;
use anodrel_wire::{FrameDecoder, WireError, encode_json};

pub const MAX_SESSION_ID_BYTES: usize = 128;
pub const SESSION_TOKEN_HEX_BYTES: usize = 64;
const AUTHENTICATE_KIND: &str = "session.authenticate";
const AUTHENTICATED_KIND: &str = "session.authenticated";
const CANCELLATION_KIND: &str = "cancel";
/// A session retains at most this many controls that arrived before their
/// corresponding request. The bounded set prevents cancellation-only traffic
/// from becoming unbounded host memory.
pub const MAX_PENDING_CANCELLATIONS: usize = 32;

#[derive(Debug)]
struct TransportUnavailableDiagnostics;

impl DiagnosticsService for TransportUnavailableDiagnostics {
    fn entries(
        &self,
    ) -> Result<Vec<anodrel_diagnostics::Entry>, anodrel_diagnostics::DiagnosticsServiceError> {
        Err(anodrel_diagnostics::DiagnosticsServiceError::Unavailable)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialsError {
    InvalidSessionId,
    InvalidToken,
}

impl fmt::Display for CredentialsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSessionId => write!(formatter, "session ID is invalid"),
            Self::InvalidToken => write!(formatter, "session token is invalid"),
        }
    }
}

impl std::error::Error for CredentialsError {}

/// Host-created material required before a stream can issue protocol requests.
/// The secret is intentionally not exposed through a getter or `Debug`.
pub struct SessionCredentials {
    session_id: String,
    token: Vec<u8>,
}

impl SessionCredentials {
    pub fn new(
        session_id: impl Into<String>,
        token: impl AsRef<str>,
    ) -> Result<Self, CredentialsError> {
        let session_id = session_id.into();
        if !is_valid_session_id(&session_id) {
            return Err(CredentialsError::InvalidSessionId);
        }
        let token = token.as_ref();
        if !is_valid_token(token) {
            return Err(CredentialsError::InvalidToken);
        }
        Ok(Self {
            session_id,
            token: token.as_bytes().to_vec(),
        })
    }
}

impl fmt::Debug for SessionCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionCredentials")
            .field("session_id", &self.session_id)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl Drop for SessionCredentials {
    fn drop(&mut self) {
        self.token.fill(0);
    }
}

/// Builds the first control payload from invitation material. The caller must
/// pass it only through the authenticated private bootstrap channel.
pub fn authentication_message(session_id: &str, token: &str) -> Result<String, CredentialsError> {
    if !is_valid_session_id(session_id) {
        return Err(CredentialsError::InvalidSessionId);
    }
    if !is_valid_token(token) {
        return Err(CredentialsError::InvalidToken);
    }
    Ok(object([
        ("kind", JsonValue::String(AUTHENTICATE_KIND.to_owned())),
        ("sessionId", JsonValue::String(session_id.to_owned())),
        ("token", JsonValue::String(token.to_owned())),
    ])
    .to_json())
}

#[derive(Debug)]
pub enum TransportError {
    Wire(WireError),
    AuthenticationRequired,
    AuthenticationFailed,
    CancellationInvalid,
    CancellationLimitReached,
    SessionClosed,
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wire(error) => write!(formatter, "native transport error: {error}"),
            Self::AuthenticationRequired => {
                write!(formatter, "native session authentication is required")
            }
            Self::AuthenticationFailed => write!(formatter, "native session authentication failed"),
            Self::CancellationInvalid => {
                write!(formatter, "native cancellation control is invalid")
            }
            Self::CancellationLimitReached => {
                write!(
                    formatter,
                    "native session reached its pending cancellation limit"
                )
            }
            Self::SessionClosed => write!(formatter, "native transport session is closed"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<WireError> for TransportError {
    fn from(error: WireError) -> Self {
        Self::Wire(error)
    }
}

#[derive(Debug)]
enum SessionState {
    Pending(SessionCredentials),
    Authenticated,
    Closed,
}

#[derive(Debug)]
struct TransportUnavailableClipboard;

impl ClipboardService for TransportUnavailableClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }

    fn write_text(&self, _text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct TransportUnavailableExternalLinks;

impl ExternalLinkService for TransportUnavailableExternalLinks {
    fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        Err(ExternalLinkOpenError::Unavailable)
    }
}

#[derive(Debug)]
struct TransportUnavailableFileDialogs;

impl FileDialogService for TransportUnavailableFileDialogs {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct TransportUnavailableStorage;

impl StorageService for TransportUnavailableStorage {
    fn read(&self) -> Result<anodrel_storage::StorageRead, anodrel_storage::StorageServiceError> {
        Err(anodrel_storage::StorageServiceError::Unavailable)
    }

    fn replace(
        &self,
        _snapshot: &anodrel_storage::StorageSnapshot,
    ) -> Result<(), anodrel_storage::StorageServiceError> {
        Err(anodrel_storage::StorageServiceError::Unavailable)
    }

    fn clear(&self) -> Result<(), anodrel_storage::StorageServiceError> {
        Err(anodrel_storage::StorageServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct TransportUnavailableCredentials;

impl CredentialService for TransportUnavailableCredentials {
    fn read(&self, _name: &CredentialName) -> Result<Secret, CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }

    fn write(
        &self,
        _name: &CredentialName,
        _secret: &Secret,
    ) -> Result<(), CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }

    fn delete(&self, _name: &CredentialName) -> Result<bool, CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }
}

fn is_valid_session_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_SESSION_ID_BYTES
}

fn is_valid_token(value: &str) -> bool {
    value.len() == SESSION_TOKEN_HEX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn constant_time_equals(candidate: &[u8], expected: &[u8]) -> bool {
    let mut difference = candidate.len() ^ expected.len();
    for (index, expected_byte) in expected.iter().enumerate() {
        difference |= usize::from(candidate.get(index).copied().unwrap_or(0) ^ expected_byte);
    }
    difference == 0
}

#[cfg(test)]
mod tests;
