#![deny(unsafe_op_in_unsafe_fn)]

//! One-client, authenticated Windows named-pipe adapter for Anodrel transport.
//!
//! `serve_one` performs synchronous I/O and is intentionally a worker-thread
//! operation. It must never be called from the Win32 UI thread.

mod loopback;
mod raw;
mod security;

mod server;

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
use anodrel_ui_session::UiWindowGroup;
use anodrel_window::WindowTitleProposal;
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
mod tests;
