//! Explicit native-service composition constructors for one pipe session.
//!
//! These compatibility builders retain their individual host-owned service
//! seams. Pipe creation and blocking I/O stay in the parent server module.

use super::*;

impl WindowsPipeServer {
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
}
