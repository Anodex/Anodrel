//! Named-pipe endpoint construction for registered Windows applications.

use std::{fmt, io};

use anodrel_core::{HostPolicy, HostServices};
use anodrel_session_policy::host_policy_for_installed_application;
use anodrel_windows_pipe::{SessionInvitation, WindowsPipeServer};
use anodrel_windows_policy::load_installed_application;

use crate::{
    RegisteredSessionError, RegisteredSessionUi,
    services::{registered_interactive_services, registered_services},
};

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
    let ui = RegisteredSessionUi::with_update_action(
        application.identity().display_name(),
        application
            .update_catalogue_location()
            .map(|_| application.identity().application_id()),
    );
    let services = registered_interactive_services(&application, &ui)?;
    let (server, invitation) =
        WindowsPipeServer::create_with_session_window_group_and_service_bundle(
            policy,
            session_id,
            ui.window_group(),
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
pub(crate) fn create_session(
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
