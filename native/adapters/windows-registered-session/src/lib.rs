#![forbid(unsafe_code)]

//! Windows registered-application session setup.
//!
//! This adapter joins a validated, machine-selected application policy to one
//! owner-restricted Windows named-pipe session. Its optional interactive path
//! also creates the one grouped set of host-owned resources consumed by an
//! authenticated native window. It does not launch a process, deliver a
//! bootstrap invitation, select an application ID, or serve pipe I/O; callers
//! retain those lifecycle responsibilities.

use std::{fmt, io};

use anodrel_application::InstalledApplication;
use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_file_access::SelectionFileDialogMailbox;
use anodrel_file_dialog::FileDialogMailbox;
use anodrel_session_policy::host_policy_for_installed_application;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_clipboard::WindowsClipboard;
use anodrel_windows_credentials::WindowsCredentialService;
use anodrel_windows_external_links::WindowsExternalLinks;
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_paths::{WindowsPathsError, application_directories};
use anodrel_windows_pipe::{SessionInvitation, WindowsPipeServer};
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_storage::WindowsStorageService;

/// The host-owned native UI resources for one registered application session.
///
/// This group has no native handle, process, title, launch operation, or
/// application-selected data. Host code must pass it only to the native window
/// that belongs to the same session returned by [`RegisteredUiSession`].
#[derive(Clone, Debug)]
pub struct RegisteredSessionUi {
    document_mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
}

impl RegisteredSessionUi {
    fn new() -> Self {
        Self {
            document_mailbox: UiDocumentMailbox::new(),
            input_mailbox: UiInputMailbox::new(),
            close_signal: SessionCloseSignal::default(),
            file_dialog_mailbox: FileDialogMailbox::new(),
            file_text: WindowsFileTextService::new(),
        }
    }

    /// Returns this session's document-delivery mailbox.
    #[must_use]
    pub fn document_mailbox(&self) -> UiDocumentMailbox {
        self.document_mailbox.clone()
    }

    /// Returns this session's bounded semantic-input mailbox.
    #[must_use]
    pub fn input_mailbox(&self) -> UiInputMailbox {
        self.input_mailbox.clone()
    }

    /// Returns this session's host-owned close signal.
    #[must_use]
    pub fn close_signal(&self) -> SessionCloseSignal {
        self.close_signal.clone()
    }

    /// Returns this session's one-request UI-thread dialog mailbox.
    #[must_use]
    pub fn file_dialog_mailbox(&self) -> FileDialogMailbox {
        self.file_dialog_mailbox.clone()
    }

    /// Returns this session's retained selected-file text service.
    #[must_use]
    pub fn file_text_service(&self) -> WindowsFileTextService {
        self.file_text.clone()
    }
}

/// One registered interactive session before a host starts pipe I/O.
///
/// The pipe endpoint, sensitive invitation, and UI group are created together
/// from the same validated machine policy record. Host code must keep them in
/// the same launch and shutdown lifecycle.
pub struct RegisteredUiSession {
    server: WindowsPipeServer,
    invitation: SessionInvitation,
    ui: RegisteredSessionUi,
}

impl RegisteredUiSession {
    /// Separates the endpoint and invitation from the grouped native UI
    /// resources for explicit host lifecycle ownership.
    #[must_use]
    pub fn into_parts(self) -> (WindowsPipeServer, SessionInvitation, RegisteredSessionUi) {
        (self.server, self.invitation, self.ui)
    }
}

impl fmt::Debug for RegisteredUiSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RegisteredUiSession(..)")
    }
}

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

/// Creates one interactive session for a host-selected registered application.
///
/// The returned resources are attached to the authenticated transport before
/// the pipe peer connects. This does not start a process, create a window, or
/// deliver the invitation; a verified host lifecycle owns those steps.
pub fn create_registered_ui_session(
    application_id: &str,
    host_name: impl Into<String>,
    session_id: impl Into<String>,
) -> Result<RegisteredUiSession, RegisteredSessionError> {
    let application =
        load_installed_application(application_id).map_err(RegisteredSessionError::Policy)?;
    let policy = host_policy_for_installed_application(&application, host_name)
        .map_err(|_| RegisteredSessionError::InvalidHostName)?;
    let ui = RegisteredSessionUi::new();
    let services = registered_interactive_services(&application, &ui)?;
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            session_id,
            ui.document_mailbox(),
            ui.input_mailbox(),
            ui.close_signal(),
            services,
        )
        .map_err(RegisteredSessionError::Pipe)?;
    Ok(RegisteredUiSession {
        server,
        invitation,
        ui,
    })
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

fn registered_interactive_services(
    application: &InstalledApplication,
    ui: &RegisteredSessionUi,
) -> Result<HostServices, RegisteredSessionError> {
    Ok(registered_services(application)?
        .with_file_dialogs(ui.file_dialog_mailbox())
        .with_file_selections(SelectionFileDialogMailbox::new(ui.file_dialog_mailbox()))
        .with_file_text(ui.file_text_service()))
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

    use super::{
        RegisteredSessionError, RegisteredSessionUi, create_registered_session, create_session,
    };

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

    #[test]
    fn interactive_ui_resources_keep_their_close_signal_session_local() {
        let first = RegisteredSessionUi::new();
        let second = RegisteredSessionUi::new();

        first.close_signal().request();
        assert!(first.close_signal().take());
        assert!(!second.close_signal().take());
    }
}
