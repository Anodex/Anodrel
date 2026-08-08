#![forbid(unsafe_code)]

//! A policy-bound, authenticated, byte-stream-independent native host session.
//!
//! OS adapters own invitation delivery and blocking I/O. This module owns the
//! bounded transition from framed input to complete core responses and refuses
//! every public protocol request until host-created credentials are verified.

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

#[derive(Debug)]
pub struct TransportSession {
    decoder: FrameDecoder,
    host: CoreHost,
    ui_document_mailbox: UiDocumentMailbox,
    pending_cancellations: BTreeSet<String>,
    state: SessionState,
}

impl TransportSession {
    /// Creates one session with both host-issued policy and host-created
    /// credentials. Stream input cannot modify either after construction.
    pub fn new(policy: HostPolicy, credentials: SessionCredentials) -> Self {
        Self::with_ui_document_mailbox(policy, credentials, UiDocumentMailbox::new())
    }

    /// Creates a session from one complete native service bundle. The caller
    /// supplies every operating-system service before authentication begins;
    /// protocol traffic cannot mutate this composition.
    #[must_use]
    pub fn with_services(
        policy: HostPolicy,
        credentials: SessionCredentials,
        services: HostServices,
    ) -> Self {
        Self {
            decoder: FrameDecoder::new(),
            host: CoreHost::with_services(policy, services),
            ui_document_mailbox: UiDocumentMailbox::new(),
            pending_cancellations: BTreeSet::new(),
            state: SessionState::Pending(credentials),
        }
    }

    /// Creates an authenticated session with only an identity-bound credential
    /// service enabled. Other platform services remain unavailable.
    pub fn with_credential_service(
        policy: HostPolicy,
        credentials: SessionCredentials,
        credential_service: impl CredentialService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
            policy,
            credentials,
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            TransportUnavailableClipboard,
            TransportUnavailableExternalLinks,
            TransportUnavailableFileDialogs,
            anodrel_file_access::UnavailableFileSelectionService,
            anodrel_file_access::UnavailableFileTextService,
            TransportUnavailableStorage,
            TransportUnavailableDiagnostics,
            credential_service,
        )
    }

    /// Creates one session that publishes accepted UI documents into one
    /// caller-owned bounded mailbox.
    pub fn with_ui_document_mailbox(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
    ) -> Self {
        Self::with_ui_mailboxes(
            policy,
            credentials,
            ui_document_mailbox,
            UiInputMailbox::new(),
        )
    }

    /// Creates one session with explicit bounded document and semantic-input
    /// mailboxes for its host-controlled native view.
    pub fn with_ui_mailboxes(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
    ) -> Self {
        Self::with_session_components(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            SessionCloseSignal::default(),
        )
    }

    /// Creates one session with explicit native UI and lifecycle components.
    pub fn with_session_components(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
    ) -> Self {
        Self::with_session_components_and_clipboard(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            TransportUnavailableClipboard,
        )
    }

    /// Creates one session with explicit native components and one portable
    /// clipboard service supplied by the native host.
    pub fn with_session_components_and_clipboard(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
    ) -> Self {
        Self::with_session_components_and_services(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            TransportUnavailableExternalLinks,
        )
    }

    /// Creates one session with explicit native components plus injected
    /// portable clipboard and external-link services.
    pub fn with_session_components_and_services(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            TransportUnavailableFileDialogs,
        )
    }

    /// Creates one session with all injected platform services.
    #[allow(clippy::too_many_arguments)] // Explicit per-session native service seams stay visible.
    pub fn with_session_components_and_all_services(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            anodrel_file_access::UnavailableFileSelectionService,
            anodrel_file_access::UnavailableFileTextService,
        )
    }

    /// Creates one session with explicit selection-capture and selected-file
    /// text services in addition to the existing native service seams.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            file_selections,
            file_text,
            TransportUnavailableStorage,
        )
    }

    /// Creates one session with an explicit host-owned application-state store.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
        storage: impl StorageService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            file_selections,
            file_text,
            storage,
            TransportUnavailableDiagnostics,
        )
    }

    /// Creates one session with an explicit bounded host diagnostics source.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
        storage: impl StorageService + 'static,
        diagnostics: impl DiagnosticsService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            file_selections,
            file_text,
            storage,
            diagnostics,
            TransportUnavailableCredentials,
        )
    }

    /// Creates one session with an identity-bound credential service supplied
    /// by the native host. The service owns application identity and target
    /// selection; the pipe peer can supply only a validated local name.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
        storage: impl StorageService + 'static,
        diagnostics: impl DiagnosticsService + 'static,
        credential_service: impl CredentialService + 'static,
    ) -> Self {
        Self {
            decoder: FrameDecoder::new(),
            host: CoreHost::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
                policy,
                ui_input_mailbox,
                session_close_signal,
                clipboard,
                external_links,
                file_dialogs,
                file_selections,
                file_text,
                storage,
                diagnostics,
                credential_service,
            ),
            ui_document_mailbox,
            pending_cancellations: BTreeSet::new(),
            state: SessionState::Pending(credentials),
        }
    }

    /// Accepts arbitrary chunks from one byte stream and returns complete
    /// response frames in arrival order. Any error is terminal; the OS adapter
    /// must close its stream instead of retrying or resynchronizing.
    pub fn receive(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, TransportError> {
        if matches!(self.state, SessionState::Closed) {
            return Err(TransportError::SessionClosed);
        }
        let requests = match self.decoder.push(bytes) {
            Ok(requests) => requests,
            Err(error) => return self.close_with(TransportError::Wire(error)),
        };
        let mut responses = Vec::with_capacity(requests.len());
        for request in requests {
            if let Some(response) = self.handle_message(&request)? {
                responses.push(self.encode_or_close(response)?);
            }
        }
        Ok(responses)
    }

    fn handle_message(&mut self, message: &str) -> Result<Option<String>, TransportError> {
        match &self.state {
            SessionState::Pending(credentials) => {
                if !matches_credentials(message, credentials) {
                    return self.close_with(TransportError::AuthenticationFailed);
                }
                self.state = SessionState::Authenticated;
                Ok(Some(
                    object([("kind", JsonValue::String(AUTHENTICATED_KIND.to_owned()))]).to_json(),
                ))
            }
            SessionState::Authenticated if has_kind(message, AUTHENTICATE_KIND) => {
                self.close_with(TransportError::AuthenticationFailed)
            }
            SessionState::Authenticated => {
                if has_kind(message, CANCELLATION_KIND) {
                    self.remember_cancellation(message)?;
                    return Ok(None);
                }
                if let Some(request) =
                    request_with_pending_cancellation(message, &mut self.pending_cancellations)
                {
                    return Ok(Some(self.host.cancelled_response(request.request_id)));
                }
                let response = self.host.handle_json(message);
                if let Some(snapshot) = self.host.take_ui_document_update() {
                    self.ui_document_mailbox.publish(snapshot);
                }
                Ok(Some(response))
            }
            SessionState::Closed => Err(TransportError::SessionClosed),
        }
    }

    fn remember_cancellation(&mut self, message: &str) -> Result<(), TransportError> {
        let control = JsonValue::parse(message)
            .ok()
            .and_then(|value| CancellationEnvelope::from_json(value).ok())
            .filter(|control| control.protocol_version.is_supported());
        let Some(control) = control else {
            return self.close_with(TransportError::CancellationInvalid);
        };
        if self
            .pending_cancellations
            .contains(&control.cancellation_id)
        {
            return Ok(());
        }
        if self.pending_cancellations.len() == MAX_PENDING_CANCELLATIONS {
            return self.close_with(TransportError::CancellationLimitReached);
        }
        self.pending_cancellations.insert(control.cancellation_id);
        Ok(())
    }

    fn encode_or_close(&mut self, response: String) -> Result<Vec<u8>, TransportError> {
        match encode_json(&response) {
            Ok(frame) => Ok(frame),
            Err(error) => self.close_with(TransportError::Wire(error)),
        }
    }

    fn close_with<T>(&mut self, error: TransportError) -> Result<T, TransportError> {
        self.state = SessionState::Closed;
        Err(error)
    }
}

fn request_with_pending_cancellation(
    message: &str,
    pending_cancellations: &mut BTreeSet<String>,
) -> Option<RequestEnvelope> {
    let request = JsonValue::parse(message)
        .ok()
        .and_then(|value| RequestEnvelope::from_json(value).ok())?;
    request
        .cancellation_id
        .as_ref()
        .filter(|cancellation_id| pending_cancellations.remove(*cancellation_id))?;
    Some(request)
}

fn matches_credentials(message: &str, credentials: &SessionCredentials) -> bool {
    let Ok(value) = JsonValue::parse(message) else {
        return false;
    };
    let Some(fields) = value.as_object() else {
        return false;
    };
    if fields.len() != 3
        || fields.get("kind").and_then(JsonValue::as_string) != Some(AUTHENTICATE_KIND)
        || fields.get("sessionId").and_then(JsonValue::as_string)
            != Some(credentials.session_id.as_str())
    {
        return false;
    }
    let Some(token) = fields.get("token").and_then(JsonValue::as_string) else {
        return false;
    };
    is_valid_token(token) && constant_time_equals(token.as_bytes(), &credentials.token)
}

fn has_kind(message: &str, expected: &str) -> bool {
    JsonValue::parse(message)
        .ok()
        .and_then(|value| {
            value
                .as_object()
                .and_then(|fields| fields.get("kind").cloned())
        })
        .and_then(|value| value.as_string().map(str::to_owned))
        .as_deref()
        == Some(expected)
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
mod tests {
    use std::cell::RefCell;

    use anodrel_clipboard::{
        ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText,
    };
    use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
    use anodrel_ui::{ElementId, UiEvent};

    use anodrel_protocol::{Capability, JsonValue};
    use anodrel_wire::{FrameDecoder, WireError, encode_json};

    use super::*;

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
            Secret::new(vec![0, 0xaa, 0xff])
                .map_err(|_| CredentialServiceError::StoredSecretInvalid)
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
            transport
                .receive(&encode_json(&cancellation("one-too-many")).expect("control encodes"),),
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
                &encode_json(r#"{"protocolVersion":{"major":1,"minor":0},"kind":"cancel","cancellationId":""}"#)
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
            .receive(
                &encode_json(&clipboard_request("clipboard.read", "{}")).expect("request encodes"),
            )
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
}
