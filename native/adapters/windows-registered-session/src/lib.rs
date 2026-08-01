#![forbid(unsafe_code)]

//! Windows registered-application session setup.
//!
//! This adapter joins a validated, machine-selected application policy to one
//! owner-restricted Windows named-pipe session. It does not launch a process,
//! deliver a bootstrap invitation, select an application ID, or serve pipe I/O;
//! callers retain those lifecycle responsibilities.

use std::{fmt, io};

use anodrel_core::HostPolicy;
use anodrel_windows_pipe::{SessionInvitation, WindowsPipeServer};
use anodrel_windows_policy::{PolicyStoreError, load_host_policy};

/// Creates one authenticated named-pipe session for a host-selected registered
/// application.
///
/// `application_id`, `host_name`, and `session_id` must be supplied by native
/// host policy, never by package, bootstrap, transport, protocol, or UI data.
/// The sensitive invitation remains separate from the endpoint and must be
/// delivered through the existing private bootstrap channel only after the
/// caller has completed executable trust checks.
pub fn create_registered_session(
    application_id: &str,
    host_name: impl Into<String>,
    session_id: impl Into<String>,
) -> Result<(WindowsPipeServer, SessionInvitation), RegisteredSessionError> {
    let policy =
        load_host_policy(application_id, host_name).map_err(RegisteredSessionError::Policy)?;
    create_session(policy, session_id).map_err(RegisteredSessionError::Pipe)
}

fn create_session(
    policy: HostPolicy,
    session_id: impl Into<String>,
) -> io::Result<(WindowsPipeServer, SessionInvitation)> {
    WindowsPipeServer::create(policy, session_id)
}

/// A safe failure category while creating a registered application session.
#[derive(Debug)]
pub enum RegisteredSessionError {
    Policy(PolicyStoreError),
    Pipe(io::Error),
}

impl fmt::Display for RegisteredSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(_) => {
                formatter.write_str("registered application policy could not be loaded")
            }
            Self::Pipe(_) => {
                formatter.write_str("registered application session could not be created")
            }
        }
    }
}

impl std::error::Error for RegisteredSessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            Self::Pipe(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_core::HostPolicy;
    use anodrel_protocol::Capability;
    use anodrel_windows_policy::PolicyStoreError;

    use super::{RegisteredSessionError, create_registered_session, create_session};

    #[test]
    fn rejects_an_invalid_application_id_before_creating_a_pipe() {
        assert!(matches!(
            create_registered_session("org.anodrel/escape", "windows-host", "test-session"),
            Err(RegisteredSessionError::Policy(
                PolicyStoreError::InvalidApplicationId
            ))
        ));
    }

    #[test]
    fn creates_an_owner_restricted_session_from_a_host_policy() {
        let policy = HostPolicy::new(
            "org.anodrel.sample",
            vec![Capability::DiagnosticsRead],
            "windows-host",
        )
        .expect("fixture host policy is valid");

        let (_server, invitation) =
            create_session(policy, "test-session").expect("session is created");
        assert!(invitation.pipe_name().starts_with(r"\\.\pipe\anodrel.v1."));
        assert_eq!(invitation.session_id(), "test-session");
    }
}
