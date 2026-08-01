#![deny(unsafe_op_in_unsafe_fn)]

//! One-client, authenticated Windows named-pipe adapter for Anodrel transport.
//!
//! `serve_one` performs synchronous I/O and is intentionally a worker-thread
//! operation. It must never be called from the Win32 UI thread.

mod loopback;
mod raw;
mod security;

use std::{fmt, io, thread};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_core::HostPolicy;
use anodrel_transport::{SessionCredentials, TransportSession, authentication_message};

use crate::{raw::PIPE_BUFFER_BYTES, security::CurrentSessionSecurity};

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
    handle: raw::OwnedHandle,
    session: TransportSession,
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

impl WindowsPipeServer {
    /// Creates a random owner-restricted endpoint and its separate sensitive
    /// invitation. The caller owns secure delivery of that invitation to the
    /// application it launches.
    pub fn create(
        policy: HostPolicy,
        session_id: impl Into<String>,
    ) -> io::Result<(Self, SessionInvitation)> {
        let session_id = session_id.into();
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
        let handle = raw::create_server_pipe(&pipe_name_wide, security.attributes())?;

        Ok((
            Self {
                handle,
                session: TransportSession::new(policy, credentials),
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
        raw::connect_server(&self.handle)?;
        let result = self.serve_connected_client();
        raw::disconnect_server(&self.handle);
        result
    }

    fn serve_connected_client(&mut self) -> io::Result<()> {
        let mut read_buffer = [0_u8; PIPE_BUFFER_BYTES];
        loop {
            let bytes_read = match raw::read(&self.handle, &mut read_buffer) {
                Ok(bytes_read) => bytes_read,
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

#[cfg(test)]
mod tests {
    use anodrel_core::HostPolicy;
    use anodrel_protocol::Capability;

    use super::*;

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
}
