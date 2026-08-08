#![forbid(unsafe_code)]

//! Windows registered-application session setup.
//!
//! This adapter joins a validated, machine-selected application policy to one
//! owner-restricted Windows named-pipe session. It does not launch a process,
//! deliver a bootstrap invitation, select an application ID, or serve pipe I/O;
//! callers retain those lifecycle responsibilities.

use std::{fmt, io};

use anodrel_application::InstalledApplication;
use anodrel_core::{HostPolicy, HostServices};
use anodrel_session_policy::host_policy_for_installed_application;
use anodrel_windows_clipboard::WindowsClipboard;
use anodrel_windows_credentials::WindowsCredentialService;
use anodrel_windows_external_links::WindowsExternalLinks;
use anodrel_windows_paths::{WindowsPathsError, application_directories};
use anodrel_windows_pipe::{SessionInvitation, WindowsPipeServer};
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_storage::WindowsStorageService;

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
    let application =
        load_installed_application(application_id).map_err(RegisteredSessionError::Policy)?;
    let policy = host_policy_for_installed_application(&application, host_name)
        .map_err(|_| RegisteredSessionError::InvalidHostName)?;
    let services = registered_services(&application)?;
    create_session_with_services(policy, session_id, services).map_err(RegisteredSessionError::Pipe)
}

#[cfg(test)]
fn create_session(
    policy: HostPolicy,
    session_id: impl Into<String>,
) -> io::Result<(WindowsPipeServer, SessionInvitation)> {
    WindowsPipeServer::create(policy, session_id)
}

fn create_session_with_services(
    policy: HostPolicy,
    session_id: impl Into<String>,
    services: HostServices,
) -> io::Result<(WindowsPipeServer, SessionInvitation)> {
    WindowsPipeServer::create_with_services(policy, session_id, services)
}

fn registered_services(
    application: &InstalledApplication,
) -> Result<HostServices, RegisteredSessionError> {
    let directories = application_directories(application.identity())
        .map_err(RegisteredSessionError::Directories)?;
    Ok(HostServices::unavailable()
        .with_clipboard(WindowsClipboard::new(0))
        .with_external_links(WindowsExternalLinks)
        .with_storage(WindowsStorageService::new(&directories))
        .with_credentials(WindowsCredentialService::new(
            application.identity().clone(),
        )))
}

/// A safe failure category while creating a registered application session.
#[derive(Debug)]
pub enum RegisteredSessionError {
    Policy(PolicyStoreError),
    InvalidHostName,
    Directories(WindowsPathsError),
    Pipe(io::Error),
}

impl fmt::Display for RegisteredSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(_) => {
                formatter.write_str("registered application policy could not be loaded")
            }
            Self::InvalidHostName => {
                formatter.write_str("registered application session host name is invalid")
            }
            Self::Directories(_) => formatter
                .write_str("registered application service directories could not be derived"),
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
            Self::Directories(error) => Some(error),
            Self::Pipe(error) => Some(error),
            Self::InvalidHostName => None,
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
