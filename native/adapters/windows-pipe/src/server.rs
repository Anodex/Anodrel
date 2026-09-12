//! Native Windows named-pipe lifecycle and authenticated worker setup.
//!
//! This module retains pipe handles, access-control checks, endpoint creation,
//! and worker shutdown. Legacy service-composition constructors live in the
//! focused `service_builders` child module.

mod service_builders;

use super::*;

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
    /// consumed by this server, so protocol traffic cannot alter it.
    pub fn create_with_services(
        policy: HostPolicy,
        session_id: impl Into<String>,
        services: HostServices,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_services(policy, credentials, services)
        })
    }

    /// Creates an interactive endpoint from host-owned UI components and a
    /// complete service bundle. Host code separately owns the native window.
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

    /// Creates an interactive endpoint whose primary view belongs to one
    /// host-created session-owned window group.
    #[allow(clippy::too_many_arguments)] // Explicit host-owned boundaries are security-relevant.
    pub fn create_with_session_window_group_and_service_bundle(
        policy: HostPolicy,
        session_id: impl Into<String>,
        ui_window_group: UiWindowGroup<WindowTitleProposal>,
        session_close_signal: SessionCloseSignal,
        services: HostServices,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_session_window_group_and_service_bundle(
                policy,
                credentials,
                ui_window_group,
                session_close_signal,
                services,
            )
        })
    }

    /// Creates an authenticated worker endpoint with only an identity-bound
    /// credential service enabled.
    pub fn create_with_credential_service(
        policy: HostPolicy,
        session_id: impl Into<String>,
        credential_service: impl CredentialService + 'static,
    ) -> io::Result<(Self, SessionInvitation)> {
        Self::create_endpoint(session_id.into(), move |credentials| {
            TransportSession::with_credential_service(policy, credentials, credential_service)
        })
    }

    /// Creates one endpoint whose accepted document snapshots are published to
    /// the supplied per-session mailbox.
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

    /// Creates one endpoint whose view uses bounded document and input
    /// mailboxes.
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

    /// Serves one connected client to EOF on a dedicated worker thread.
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
