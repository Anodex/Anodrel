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

#[derive(Debug)]
pub struct TransportSession {
    decoder: FrameDecoder,
    host: CoreHost,
    ui_document_delivery: UiDocumentDelivery,
    pending_cancellations: BTreeSet<String>,
    state: SessionState,
}

/// The host-owned route for an accepted primary document snapshot.
///
/// Legacy sessions publish through one standalone mailbox after core handling.
/// A session-owned group publishes directly into the view's mailbox while it
/// holds its own synchronized state, so the transport must not publish a
/// second copy afterward.
#[derive(Debug)]
enum UiDocumentDelivery {
    Legacy(UiDocumentMailbox),
    Group,
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
            ui_document_delivery: UiDocumentDelivery::Legacy(UiDocumentMailbox::new()),
            pending_cancellations: BTreeSet::new(),
            state: SessionState::Pending(credentials),
        }
    }

    /// Creates an interactive session from host-owned UI components and one
    /// complete native service bundle. The components are fixed before the
    /// peer authenticates and cannot be selected through protocol traffic.
    #[must_use]
    pub fn with_session_components_and_service_bundle(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        services: HostServices,
    ) -> Self {
        Self {
            decoder: FrameDecoder::new(),
            host: CoreHost::with_session_components_and_service_bundle(
                policy,
                ui_input_mailbox,
                session_close_signal,
                services,
            ),
            ui_document_delivery: UiDocumentDelivery::Legacy(ui_document_mailbox),
            pending_cancellations: BTreeSet::new(),
            state: SessionState::Pending(credentials),
        }
    }

    /// Creates a session whose primary view belongs to one portable
    /// session-owned window group.
    ///
    /// The group is created by the host with its real primary view mailboxes
    /// before authentication starts. Core updates then publish through that
    /// group directly; the pipe worker cannot duplicate a snapshot by using
    /// the legacy document-delivery path.
    #[must_use]
    pub fn with_session_window_group_and_service_bundle(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_window_group: anodrel_ui_session::UiWindowGroup<WindowTitleProposal>,
        session_close_signal: SessionCloseSignal,
        services: HostServices,
    ) -> Self {
        Self {
            decoder: FrameDecoder::new(),
            host: CoreHost::with_session_window_group_and_service_bundle(
                policy,
                ui_window_group,
                session_close_signal,
                services,
            ),
            ui_document_delivery: UiDocumentDelivery::Group,
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
            ui_document_delivery: UiDocumentDelivery::Legacy(ui_document_mailbox),
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
                if let UiDocumentDelivery::Legacy(mailbox) = &self.ui_document_delivery
                    && let Some(snapshot) = self.host.take_ui_document_update()
                {
                    mailbox.publish(snapshot);
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
mod tests;
