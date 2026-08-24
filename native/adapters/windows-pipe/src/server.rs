//! Native Windows named-pipe server lifecycle and authenticated worker setup.
//!
//! This module retains all pipe handles, access-control checks, and worker
//! shutdown rules behind the adapter's explicit server API.

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

    /// Creates an authenticated interactive endpoint whose primary view belongs
    /// to one host-created session-owned window group.
    ///
    /// The group contains portable state and mailboxes only. Native host code
    /// remains responsible for servicing its creation requests on the owning
    /// UI thread and for retaining its lifetime with the associated window.
    #[allow(clippy::too_many_arguments)] // The explicit host-owned boundaries are security-relevant.
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
