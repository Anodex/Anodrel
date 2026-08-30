//! Linux-only endpoint creation and host-owned invitation state.

mod endpoint;
mod loopback;
mod server;

use std::{
    fmt, io,
    os::unix::net::{SocketAddr, UnixListener, UnixStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anodrel_core::HostPolicy;
use anodrel_transport::{SessionCredentials, TransportSession, authentication_message};

pub use loopback::run_health_self_test;

const ENDPOINT_PREFIX: &str = "anodrel.v1.";

/// Host-created material for one local Linux transport endpoint.
pub struct SessionInvitation {
    endpoint_name: String,
    endpoint: SocketAddr,
    session_id: String,
    token: Vec<u8>,
}

impl SessionInvitation {
    /// Builds the sensitive first authentication control without exposing a
    /// reusable token getter.
    pub fn authentication_payload(&self) -> Result<String, InvitationError> {
        let token = std::str::from_utf8(&self.token).map_err(|_| InvitationError::InvalidToken)?;
        authentication_message(&self.session_id, token).map_err(InvitationError::Credentials)
    }

    pub(crate) fn connect(&self) -> io::Result<UnixStream> {
        endpoint::connect(&self.endpoint)
    }
}

impl fmt::Debug for SessionInvitation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionInvitation")
            .field("endpoint_name", &self.endpoint_name)
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

/// Invitation construction errors that are safe to report only to a host.
#[derive(Debug)]
pub enum InvitationError {
    InvalidToken,
    Credentials(anodrel_transport::CredentialsError),
}

impl fmt::Display for InvitationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken => write!(formatter, "session invitation token became invalid"),
            Self::Credentials(error) => write!(formatter, "session invitation is invalid: {error}"),
        }
    }
}

impl std::error::Error for InvitationError {}

/// One host-owned abstract Unix listener and its policy-bound session.
pub struct LinuxPipeServer {
    listener: Option<UnixListener>,
    endpoint: SocketAddr,
    stop_requested: Arc<AtomicBool>,
    session: TransportSession,
}

/// Host-only signal that ends one pending or connected Linux transport worker.
#[derive(Clone)]
pub struct LinuxPipeStopSignal {
    endpoint: SocketAddr,
    requested: Arc<AtomicBool>,
}

impl fmt::Debug for LinuxPipeStopSignal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LinuxPipeStopSignal(..)")
    }
}

impl LinuxPipeStopSignal {
    /// Requests a best-effort stop without creating protocol traffic.
    pub fn request_stop(&self) {
        self.requested.store(true, Ordering::Release);
        // A private local connection wakes a worker polling its one pending
        // accept. A connected worker observes this bit through its short read
        // timeout instead.
        let _ = endpoint::connect(&self.endpoint);
    }
}

impl LinuxPipeServer {
    /// Creates one random, same-UID, authenticated abstract Unix endpoint.
    pub fn create(
        policy: HostPolicy,
        session_id: impl Into<String>,
    ) -> io::Result<(Self, SessionInvitation)> {
        let endpoint_name = endpoint::random_hex()?;
        let endpoint_name = format!("{ENDPOINT_PREFIX}{endpoint_name}");
        let endpoint = endpoint::abstract_address(&endpoint_name)?;
        let listener = UnixListener::bind_addr(&endpoint)?;
        listener.set_nonblocking(true)?;

        let token = endpoint::random_hex()?;
        let session_id = session_id.into();
        let credentials = SessionCredentials::new(session_id.clone(), &token).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot create session credentials: {error}"),
            )
        })?;
        let stop_requested = Arc::new(AtomicBool::new(false));

        Ok((
            Self {
                listener: Some(listener),
                endpoint: endpoint.clone(),
                stop_requested,
                session: TransportSession::new(policy, credentials),
            },
            SessionInvitation {
                endpoint_name,
                endpoint,
                session_id,
                token: token.into_bytes(),
            },
        ))
    }

    /// Returns a host-only signal for this endpoint's worker.
    #[must_use]
    pub fn stop_signal(&self) -> LinuxPipeStopSignal {
        LinuxPipeStopSignal {
            endpoint: self.endpoint.clone(),
            requested: Arc::clone(&self.stop_requested),
        }
    }
}

#[cfg(test)]
mod tests;
