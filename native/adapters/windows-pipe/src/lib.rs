#![deny(unsafe_op_in_unsafe_fn)]

//! One-client, authenticated Windows named-pipe adapter for Anodrel transport.
//!
//! `serve_one` performs synchronous I/O and is intentionally a worker-thread
//! operation. It must never be called from the Win32 UI thread.

mod loopback;
mod raw;
mod security;

use std::{
    fmt, io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_clipboard::ClipboardService;
use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_credentials::{CredentialName, CredentialService, CredentialServiceError, Secret};
use anodrel_diagnostics::DiagnosticsService;
use anodrel_external_links::ExternalLinkService;
use anodrel_file_access::{FileSelectionService, FileTextService};
use anodrel_file_dialog::{
    FileDialogFilter, FileDialogSelection, FileDialogService, FileDialogServiceError,
};
use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};
use anodrel_transport::{
    SessionCredentials, TransportSession, UiDocumentMailbox, UiInputMailbox, authentication_message,
};
use anodrel_wire::encode_json;

use crate::{raw::PIPE_BUFFER_BYTES, security::CurrentSessionSecurity};

#[derive(Debug)]
struct UnavailableFileDialogs;

impl FileDialogService for UnavailableFileDialogs {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableStorage;

impl StorageService for UnavailableStorage {
    fn read(&self) -> Result<StorageRead, StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }
    fn replace(&self, _snapshot: &StorageSnapshot) -> Result<(), StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }
    fn clear(&self) -> Result<(), StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableDiagnostics;

impl DiagnosticsService for UnavailableDiagnostics {
    fn entries(
        &self,
    ) -> Result<Vec<anodrel_diagnostics::Entry>, anodrel_diagnostics::DiagnosticsServiceError> {
        Err(anodrel_diagnostics::DiagnosticsServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableCredentials;

impl CredentialService for UnavailableCredentials {
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

pub struct SessionInvitation {
    pipe_name: String,
    session_id: String,
    token: Vec<u8>,
}

impl SessionInvitation {
    pub fn pipe_name(&self) -> &str {
        &self.pipe_name
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Produces the sensitive first control payload for a future private
    /// application bootstrap. Do not put this payload in command lines,
    /// environment variables, logs, or predictable files.
    pub fn authentication_payload(&self) -> Result<String, InvitationError> {
        let token = std::str::from_utf8(&self.token).map_err(|_| InvitationError::InvalidToken)?;
        authentication_message(&self.session_id, token).map_err(InvitationError::Credentials)
    }

    /// Converts this pipe invitation into the bounded one-use child-bootstrap
    /// record. The token remains private and is never exposed through a getter.
    pub fn bootstrap_invitation(&self) -> Result<BootstrapInvitation, InvitationError> {
        let token = std::str::from_utf8(&self.token).map_err(|_| InvitationError::InvalidToken)?;
        BootstrapInvitation::new(&self.pipe_name, &self.session_id, token)
            .map_err(InvitationError::Bootstrap)
    }
}

impl fmt::Debug for SessionInvitation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionInvitation")
            .field("pipe_name", &self.pipe_name)
            .field("session_id", &self.session_id)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl Drop for SessionInvitation {
    fn drop(&mut self) {
        self.token.fill(0);
    }
}

#[derive(Debug)]
pub enum InvitationError {
    InvalidToken,
    Credentials(anodrel_transport::CredentialsError),
    Bootstrap(anodrel_bootstrap::BootstrapError),
}

impl fmt::Display for InvitationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken => write!(formatter, "session invitation token became invalid"),
            Self::Credentials(error) => write!(formatter, "session invitation is invalid: {error}"),
            Self::Bootstrap(error) => write!(formatter, "bootstrap invitation is invalid: {error}"),
        }
    }
}

impl std::error::Error for InvitationError {}

pub struct WindowsPipeServer {
    handle: Arc<raw::OwnedHandle>,
    pipe_name: String,
    stop_requested: Arc<AtomicBool>,
    session: TransportSession,
}

/// Host-only signal that stops one pending or connected pipe worker.
#[derive(Clone)]
pub struct PipeStopSignal {
    handle: Arc<raw::OwnedHandle>,
    pipe_name: String,
    requested: Arc<AtomicBool>,
}

impl fmt::Debug for PipeStopSignal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PipeStopSignal(..)")
    }
}

impl PipeStopSignal {
    /// Requests a best-effort stop for this one host-owned pipe worker.
    pub fn request_stop(&self) {
        self.requested.store(true, Ordering::Release);
        // A private local connection wakes a server that has not entered its
        // blocking accept yet; cancellation wakes an already pending accept or
        // read. Neither result is application-visible.
        let name = wide_null(&self.pipe_name);
        let _ = raw::connect_client_once(&name);
        raw::cancel_pending_io(&self.handle);
    }
}

/// Runs one internal local authentication and `platform.health` round trip.
///
/// The server remains a one-client, current-session named pipe. This helper
/// creates a temporary endpoint, connects a private in-process client before
/// the worker begins its accept loop, then closes both endpoints after the
/// check. It never exposes the invitation, launches an application, or grants
/// authority beyond the caller-supplied policy.
pub fn run_health_self_test(policy: HostPolicy) -> io::Result<()> {
    let (server, invitation) = WindowsPipeServer::create(policy, "startup-lab-loopback")
        .map_err(|_| self_test_failed())?;
    let pipe_name = wide_null(invitation.pipe_name());
    let client = raw::connect_client(&pipe_name).map_err(|_| self_test_failed())?;
    let server_thread = thread::spawn(move || server.serve_one());

    let client_result = loopback::authenticated_health(&client, &invitation);
    drop(client);
    drop(invitation);

    let server_result = server_thread
        .join()
        .map_err(|_| self_test_failed())?
        .map_err(|_| self_test_failed());
    client_result?;
    server_result
}

/// Runs one private local authenticated cancellation round trip.
///
/// This verifies the real Windows pipe carries a response-less cancellation
/// control followed by one pre-execution cancelled request. It is a native
/// adapter smoke test, not an application-facing client API.
#[cfg(test)]
fn run_cancellation_self_test(policy: HostPolicy) -> io::Result<()> {
    let (server, invitation) = WindowsPipeServer::create(policy, "cancellation-loopback")
        .map_err(|_| self_test_failed())?;
    let pipe_name = wide_null(invitation.pipe_name());
    let client = raw::connect_client(&pipe_name).map_err(|_| self_test_failed())?;
    let server_thread = thread::spawn(move || server.serve_one());

    let client_result = loopback::authenticated_cancelled_health(&client, &invitation);
    drop(client);
    drop(invitation);

    let server_result = server_thread
        .join()
        .map_err(|_| self_test_failed())?
        .map_err(|_| self_test_failed());
    client_result?;
    server_result
}

/// Measures repeated local request/response round trips through one temporary
/// owner-restricted named pipe.
///
/// This is a development diagnostic, not an application-facing transport API.
/// It authenticates a private in-process client before warmup, measures only
/// the supplied complete request frame's write/read cycle, and discards the
/// invitation after the check. The caller must supply a host-created policy and
/// a request that is already valid for that policy.
pub fn measure_loopback_request(
    policy: HostPolicy,
    request_json: &str,
    warmup_iterations: usize,
    measured_iterations: usize,
) -> io::Result<Vec<Duration>> {
    let total_iterations = warmup_iterations
        .checked_add(measured_iterations)
        .filter(|total| *total <= MAX_LOOPBACK_ITERATIONS)
        .ok_or_else(invalid_measurement)?;
    if measured_iterations == 0 || total_iterations == 0 {
        return Err(invalid_measurement());
    }
    let request_frame = encode_json(request_json).map_err(|_| self_test_failed())?;
    let (server, invitation) = WindowsPipeServer::create(policy, "performance-loopback")
        .map_err(|_| self_test_failed())?;
    let pipe_name = wide_null(invitation.pipe_name());
    let client = raw::connect_client(&pipe_name).map_err(|_| self_test_failed())?;
    let server_thread = thread::spawn(move || server.serve_one());

    let client_result = loopback::measure_authenticated_request(
        &client,
        &invitation,
        &request_frame,
        warmup_iterations,
        measured_iterations,
    );
    drop(client);
    drop(invitation);

    let server_result = server_thread
        .join()
        .map_err(|_| self_test_failed())?
        .map_err(|_| self_test_failed());
    let measurements = client_result?;
    server_result?;
    Ok(measurements)
}

impl WindowsPipeServer {
    /// Returns the host-only stop signal for this endpoint.
    #[must_use]
    pub fn stop_signal(&self) -> PipeStopSignal {
        PipeStopSignal {
            handle: Arc::clone(&self.handle),
            pipe_name: self.pipe_name.clone(),
            requested: Arc::clone(&self.stop_requested),
        }
    }

    /// Creates a random owner-restricted endpoint and its separate sensitive
    /// invitation. The caller owns secure delivery of that invitation to the
    /// application it launches.
    pub fn create(
        policy: HostPolicy,
        session_id: impl Into<String>,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_with_ui_document_mailbox(policy, session_id, UiDocumentMailbox::new())
    }

    /// Creates an authenticated endpoint from a complete host-owned service
    /// bundle. The bundle is fixed before the peer can authenticate and is
    /// consumed by this server, so it cannot be altered by protocol traffic.
    pub fn create_with_services(
        policy: HostPolicy,
        session_id: impl Into<String>,
        services: HostServices,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_services(policy, credentials, services)
        })
    }

    /// Creates an authenticated interactive endpoint from host-owned UI
    /// components and a complete service bundle. The native window that owns
    /// the components is selected separately by host code.
    pub fn create_with_session_components_and_service_bundle(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        services: HostServices,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_session_components_and_service_bundle(
                policy,
                credentials,
                ui_document_mailbox,
                ui_input_mailbox,
                session_close_signal,
                services,
            )
        })
    }

    /// Creates an authenticated worker-thread endpoint with only an
    /// identity-bound credential service enabled.
    pub fn create_with_credential_service(
        policy: HostPolicy,
        session_id: impl Into<String>,
        credential_service: impl CredentialService + 'static,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_credential_service(policy, credentials, credential_service)
        })
    }

    /// Creates one endpoint whose accepted UI document snapshots are published
    /// into the supplied per-session mailbox.
    ///
    /// The caller owns the mailbox's consumer and must keep it separate from
    /// the pipe worker thread and from all other sessions.
    pub fn create_with_ui_document_mailbox(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_with_ui_mailboxes(
            policy,
            session_id,
            ui_document_mailbox,
            UiInputMailbox::new(),
        )
    }

    /// Creates one endpoint whose native view uses the supplied bounded document
    /// and semantic-input mailboxes.
    pub fn create_with_ui_mailboxes(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_with_session_components(
            policy,
            session_id,
            ui_document_mailbox,
            ui_input_mailbox,
            SessionCloseSignal::default(),
        )
    }

    /// Creates one endpoint with explicit native UI and lifecycle components.
    pub fn create_with_session_components(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_session_components(
                policy,
                credentials,
                ui_document_mailbox,
                ui_input_mailbox,
                session_close_signal,
            )
        })
    }

    /// Creates one endpoint with explicit native components and one portable
    /// clipboard service supplied by the native host.
    pub fn create_with_session_components_and_clipboard(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_session_components_and_clipboard(
                policy,
                credentials,
                ui_document_mailbox,
                ui_input_mailbox,
                session_close_signal,
                clipboard,
            )
        })
    }

    /// Creates one endpoint with explicit native components and the portable
    /// services required by its authenticated application session.
    pub fn create_with_session_components_and_services(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_with_session_components_and_all_services(
            policy,
            session_id,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            UnavailableFileDialogs,
        )
    }

    /// Creates one endpoint with all platform services for its authenticated
    /// application session.
    #[allow(clippy::too_many_arguments)] // The host supplies each session-bound service explicitly.
    pub fn create_with_session_components_and_all_services(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_with_session_components_and_all_services_and_file_access(
            policy,
            session_id,
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

    /// Creates one endpoint with explicit selection-capture and selected-file
    /// text services for its authenticated application session.
    #[allow(clippy::too_many_arguments)] // Explicit per-session native service seams stay visible.
    pub fn create_with_session_components_and_all_services_and_file_access(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_with_session_components_and_all_services_and_file_access_and_storage(
            policy,
            session_id,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            file_selections,
            file_text,
            UnavailableStorage,
        )
    }

    /// Creates one endpoint with an explicit host-owned application-state store.
    #[allow(clippy::too_many_arguments)]
    pub fn create_with_session_components_and_all_services_and_file_access_and_storage(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
        storage: impl StorageService + 'static,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
            policy,
            session_id,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            file_selections,
            file_text,
            storage,
            UnavailableDiagnostics,
        )
    }

    /// Creates one endpoint with an explicit bounded host diagnostics source.
    #[allow(clippy::too_many_arguments)]
    pub fn create_with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
        policy: HostPolicy,
        session_id: impl Into<String>,
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
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
            policy,
            session_id,
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
            UnavailableCredentials,
        )
    }

    /// Creates one worker-thread pipe endpoint with an identity-bound
    /// credential service. The service is created by the native host, never by
    /// the pipe peer or bootstrap invitation.
    #[allow(clippy::too_many_arguments)]
    pub fn create_with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
        policy: HostPolicy,
        session_id: impl Into<String>,
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
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
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
                credential_service,
            )
        })
    }

    fn create_endpoint(
        session_id: String,
        create_session: impl FnOnce(SessionCredentials) -> TransportSession,
    ) -> io::Result<(Self, SessionInvitation)> {
        let pipe_name = format!(r"\\.\pipe\anodrel.v1.{}", random_hex()?);
        let token = random_hex()?;
        let credentials = SessionCredentials::new(session_id.clone(), &token).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot create session credentials: {error}"),
            )
        })?;
        let security = CurrentSessionSecurity::new()?;
        let pipe_name_wide = wide_null(&pipe_name);
        let handle = Arc::new(raw::create_server_pipe(
            &pipe_name_wide,
            security.attributes(),
        )?);

        Ok((
            Self {
                handle,
                pipe_name: pipe_name.clone(),
                stop_requested: Arc::new(AtomicBool::new(false)),
                session: create_session(credentials),
            },
            SessionInvitation {
                pipe_name,
                session_id,
                token: token.into_bytes(),
            },
        ))
    }

    /// Serves one connected client to EOF. This blocks on pipe reads, so call it
    /// only from a dedicated worker thread. Any transport failure closes the
    /// stream without exposing parser or authentication details to the client.
    pub fn serve_one(mut self) -> io::Result<()> {
        if self.stop_requested.load(Ordering::Acquire) {
            return Ok(());
        }
        match raw::connect_server(&self.handle) {
            Ok(()) => {}
            Err(error)
                if self.stop_requested.load(Ordering::Acquire)
                    && raw::is_operation_aborted(&error) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
        if self.stop_requested.load(Ordering::Acquire) {
            raw::disconnect_server(&self.handle);
            return Ok(());
        }
        let result = self.serve_connected_client();
        raw::disconnect_server(&self.handle);
        result
    }

    fn serve_connected_client(&mut self) -> io::Result<()> {
        let mut read_buffer = [0_u8; PIPE_BUFFER_BYTES];
        loop {
            let bytes_read = match raw::read(&self.handle, &mut read_buffer) {
                Ok(bytes_read) => bytes_read,
                Err(error)
                    if self.stop_requested.load(Ordering::Acquire)
                        && raw::is_operation_aborted(&error) =>
                {
                    return Ok(());
                }
                Err(error) if raw::is_broken_pipe(&error) => return Ok(()),
                Err(error) => return Err(error),
            };
            if bytes_read == 0 {
                return Ok(());
            }
            let responses = self
                .session
                .receive(&read_buffer[..bytes_read])
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::PermissionDenied, "pipe session ended")
                })?;
            for response in responses {
                raw::write_all(&self.handle, &response)?;
            }
        }
    }
}

fn random_hex() -> io::Result<String> {
    let mut bytes = [0_u8; 32];
    raw::random_bytes(&mut bytes)?;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    Ok(encoded)
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn self_test_failed() -> io::Error {
    io::Error::other("private IPC self-test did not complete")
}

const MAX_LOOPBACK_ITERATIONS: usize = 100_200;

fn invalid_measurement() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "private IPC measurement count is invalid",
    )
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anodrel_application::ApplicationManifest;
    use anodrel_core::HostPolicy;
    use anodrel_protocol::{Capability, JsonValue};
    use anodrel_windows_credentials::WindowsCredentialService;
    use anodrel_wire::{FrameDecoder, encode_json};

    use super::*;

    fn write_json(client: &raw::OwnedHandle, message: &str) {
        raw::write_all(client, &encode_json(message).expect("test request encodes"))
            .expect("test request writes");
    }

    fn read_json(client: &raw::OwnedHandle) -> JsonValue {
        let mut decoder = FrameDecoder::new();
        let mut buffer = [0_u8; PIPE_BUFFER_BYTES];
        loop {
            let count = raw::read(client, &mut buffer).expect("test response reads");
            let messages = decoder
                .push(&buffer[..count])
                .expect("test response frame decodes");
            if let Some(message) = messages.into_iter().next() {
                return JsonValue::parse(&message).expect("test response is JSON");
            }
        }
    }

    #[test]
    fn serves_an_authenticated_health_request_over_a_real_windows_pipe() {
        let policy = HostPolicy::new(
            "test.application",
            vec![Capability::DiagnosticsRead],
            "test-host",
        )
        .expect("test policy is valid");
        run_health_self_test(policy).expect("private IPC self-test succeeds");
    }

    #[test]
    fn cancels_a_not_started_request_over_a_real_windows_pipe() {
        let policy = HostPolicy::new(
            "test.application",
            vec![Capability::DiagnosticsRead],
            "test-host",
        )
        .expect("test policy is valid");
        run_cancellation_self_test(policy).expect("private cancellation self-test succeeds");
    }

    #[test]
    fn converts_a_pipe_invitation_into_a_private_bootstrap_record() {
        let policy = HostPolicy::new(
            "test.application",
            vec![Capability::DiagnosticsRead],
            "test-host",
        )
        .expect("test policy is valid");
        let (_server, invitation) =
            WindowsPipeServer::create(policy, "test-session").expect("pipe server creates");
        let bootstrap = invitation
            .bootstrap_invitation()
            .expect("bootstrap invitation is valid");
        assert_eq!(bootstrap.pipe_name(), invitation.pipe_name());
        assert_eq!(bootstrap.session_id(), invitation.session_id());
    }

    #[test]
    fn host_stop_signal_prevents_a_pending_server_from_accepting_a_client() {
        let policy = HostPolicy::new(
            "test.application",
            vec![Capability::DiagnosticsRead],
            "test-host",
        )
        .expect("test policy is valid");
        let (server, _invitation) =
            WindowsPipeServer::create(policy, "stopped-session").expect("pipe server creates");
        server.stop_signal().request_stop();
        server
            .serve_one()
            .expect("a stopped pending server returns safely");
    }

    #[test]
    fn host_stop_signal_ends_a_connected_pipe_worker() {
        let policy = HostPolicy::new(
            "test.application",
            vec![Capability::DiagnosticsRead],
            "test-host",
        )
        .expect("test policy is valid");
        let (server, invitation) = WindowsPipeServer::create(policy, "connected-stop-session")
            .expect("pipe server creates");
        let client =
            raw::connect_client(&wide_null(invitation.pipe_name())).expect("test client connects");
        let stop = server.stop_signal();
        let worker = thread::spawn(move || server.serve_one());

        thread::sleep(Duration::from_millis(10));
        stop.request_stop();
        drop(client);

        worker
            .join()
            .expect("stopped pipe worker does not panic")
            .expect("stopped pipe worker returns safely");
    }

    #[test]
    fn measures_authenticated_health_round_trips_over_a_real_windows_pipe() {
        let policy = HostPolicy::new(
            "test.application",
            vec![Capability::DiagnosticsRead],
            "test-host",
        )
        .expect("test policy is valid");
        let request = r#"{"protocolVersion":{"major":1,"minor":0},"kind":"request","requestId":"loopback-health","operation":"platform.health","payload":{}}"#;

        let measurements = measure_loopback_request(policy, request, 1, 2)
            .expect("private loopback measurement succeeds");
        assert_eq!(measurements.len(), 2);
    }

    #[test]
    fn routes_credential_requests_over_a_real_authenticated_windows_pipe() {
        let policy = HostPolicy::new(
            "anodrel.sample",
            vec![
                Capability::CredentialRead,
                Capability::CredentialWrite,
                Capability::CredentialDelete,
            ],
            "test-host",
        )
        .expect("test policy is valid");
        let credential_name = format!(
            "pipe-credential-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is after the epoch")
                .as_nanos()
        );
        let identity = ApplicationManifest::parse(
            r#"{"manifestVersion":{"major":1,"minor":0},"applicationId":"anodrel.sample","displayName":"Anodrel Sample","content":{"format":"anodrel.text.v1","path":"content/main.txt","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}}"#,
        )
        .expect("test manifest is valid")
        .identity()
        .clone();
        let (server, invitation) = WindowsPipeServer::create_with_credential_service(
            policy,
            "credential-test-session",
            WindowsCredentialService::new(identity),
        )
        .expect("credential pipe server creates");
        let name = wide_null(invitation.pipe_name());
        let server_thread = std::thread::spawn(move || server.serve_one());
        let client = raw::connect_client(&name).expect("test client connects");

        write_json(
            &client,
            &invitation
                .authentication_payload()
                .expect("test invitation authenticates"),
        );
        assert_eq!(
            read_json(&client)
                .as_object()
                .and_then(|fields| fields.get("kind"))
                .and_then(JsonValue::as_string),
            Some("session.authenticated")
        );
        write_json(
            &client,
            &format!(
                r#"{{"protocolVersion":{{"major":1,"minor":12}},"kind":"request","requestId":"credential-write","operation":"credential.write","payload":{{"name":"{credential_name}","secret":"00aaff"}}}}"#
            ),
        );
        assert_eq!(
            read_json(&client)
                .as_object()
                .and_then(|fields| fields.get("status"))
                .and_then(JsonValue::as_string),
            Some("success")
        );
        write_json(
            &client,
            &format!(
                r#"{{"protocolVersion":{{"major":1,"minor":12}},"kind":"request","requestId":"credential-read","operation":"credential.read","payload":{{"name":"{credential_name}"}}}}"#
            ),
        );
        let response = read_json(&client);
        let result = response
            .as_object()
            .and_then(|fields| fields.get("result"))
            .and_then(JsonValue::as_object)
            .expect("read response has a result");
        assert_eq!(
            result.get("secret").and_then(JsonValue::as_string),
            Some("00aaff")
        );
        write_json(
            &client,
            &format!(
                r#"{{"protocolVersion":{{"major":1,"minor":12}},"kind":"request","requestId":"credential-delete","operation":"credential.delete","payload":{{"name":"{credential_name}"}}}}"#
            ),
        );
        assert_eq!(
            read_json(&client)
                .as_object()
                .and_then(|fields| fields.get("status"))
                .and_then(JsonValue::as_string),
            Some("success")
        );
        drop(client);
        server_thread
            .join()
            .expect("test pipe worker does not panic")
            .expect("test pipe worker completes");
    }
}
