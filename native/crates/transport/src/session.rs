//! Authenticated stream session state, framing, and cancellation bookkeeping.
//!
//! This module owns the protocol session state machine while using only the
//! complete host service composition supplied before authentication begins.

use super::*;

mod runtime;

#[derive(Debug)]
pub struct TransportSession {
    decoder: FrameDecoder,
    host: CoreHost,
    ui_document_delivery: UiDocumentDelivery,
    pending_cancellations: BTreeSet<String>,
    state: SessionState,
}

/// The host-owned route for an accepted primary document snapshot.
///
/// Legacy sessions publish through one standalone mailbox after core handling.
/// A session-owned group publishes directly into the view's mailbox while it
/// holds its own synchronized state, so the transport must not publish a
/// second copy afterward.
#[derive(Debug)]
enum UiDocumentDelivery {
    Legacy(UiDocumentMailbox),
    Group,
}

impl TransportSession {
    /// Creates one session with both host-issued policy and host-created
    /// credentials. Stream input cannot modify either after construction.
    pub fn new(policy: HostPolicy, credentials: SessionCredentials) -> Self {
        Self::with_ui_document_mailbox(policy, credentials, UiDocumentMailbox::new())
    }

    /// Creates a session from one complete native service bundle. The caller
    /// supplies every operating-system service before authentication begins;
    /// protocol traffic cannot mutate this composition.
    #[must_use]
    pub fn with_services(
        policy: HostPolicy,
        credentials: SessionCredentials,
        services: HostServices,
    ) -> Self {
        Self {
            decoder: FrameDecoder::new(),
            host: CoreHost::with_services(policy, services),
            ui_document_delivery: UiDocumentDelivery::Legacy(UiDocumentMailbox::new()),
            pending_cancellations: BTreeSet::new(),
            state: SessionState::Pending(credentials),
        }
    }

    /// Creates an interactive session from host-owned UI components and one
    /// complete native service bundle. The components are fixed before the
    /// peer authenticates and cannot be selected through protocol traffic.
    #[must_use]
    pub fn with_session_components_and_service_bundle(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        services: HostServices,
    ) -> Self {
        Self {
            decoder: FrameDecoder::new(),
            host: CoreHost::with_session_components_and_service_bundle(
                policy,
                ui_input_mailbox,
                session_close_signal,
                services,
            ),
            ui_document_delivery: UiDocumentDelivery::Legacy(ui_document_mailbox),
            pending_cancellations: BTreeSet::new(),
            state: SessionState::Pending(credentials),
        }
    }

    /// Creates a session whose primary view belongs to one portable
    /// session-owned window group.
    ///
    /// The group is created by the host with its real primary view mailboxes
    /// before authentication starts. Core updates then publish through that
    /// group directly; the pipe worker cannot duplicate a snapshot by using
    /// the legacy document-delivery path.
    #[must_use]
    pub fn with_session_window_group_and_service_bundle(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_window_group: anodrel_ui_session::UiWindowGroup<WindowTitleProposal>,
        session_close_signal: SessionCloseSignal,
        services: HostServices,
    ) -> Self {
        Self {
            decoder: FrameDecoder::new(),
            host: CoreHost::with_session_window_group_and_service_bundle(
                policy,
                ui_window_group,
                session_close_signal,
                services,
            ),
            ui_document_delivery: UiDocumentDelivery::Group,
            pending_cancellations: BTreeSet::new(),
            state: SessionState::Pending(credentials),
        }
    }

    /// Creates an authenticated session with only an identity-bound credential
    /// service enabled. Other platform services remain unavailable.
    pub fn with_credential_service(
        policy: HostPolicy,
        credentials: SessionCredentials,
        credential_service: impl CredentialService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
            policy,
            credentials,
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            TransportUnavailableClipboard,
            TransportUnavailableExternalLinks,
            TransportUnavailableFileDialogs,
            anodrel_file_access::UnavailableFileSelectionService,
            anodrel_file_access::UnavailableFileTextService,
            TransportUnavailableStorage,
            TransportUnavailableDiagnostics,
            credential_service,
        )
    }

    /// Creates one session that publishes accepted UI documents into one
    /// caller-owned bounded mailbox.
    pub fn with_ui_document_mailbox(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
    ) -> Self {
        Self::with_ui_mailboxes(
            policy,
            credentials,
            ui_document_mailbox,
            UiInputMailbox::new(),
        )
    }

    /// Creates one session with explicit bounded document and semantic-input
    /// mailboxes for its host-controlled native view.
    pub fn with_ui_mailboxes(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
    ) -> Self {
        Self::with_session_components(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            SessionCloseSignal::default(),
        )
    }

    /// Creates one session with explicit native UI and lifecycle components.
    pub fn with_session_components(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
    ) -> Self {
        Self::with_session_components_and_clipboard(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            TransportUnavailableClipboard,
        )
    }

    /// Creates one session with explicit native components and one portable
    /// clipboard service supplied by the native host.
    pub fn with_session_components_and_clipboard(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
    ) -> Self {
        Self::with_session_components_and_services(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            TransportUnavailableExternalLinks,
        )
    }

    /// Creates one session with explicit native components plus injected
    /// portable clipboard and external-link services.
    pub fn with_session_components_and_services(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services(
            policy,
            credentials,
            ui_document_mailbox,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            TransportUnavailableFileDialogs,
        )
    }

    /// Creates one session with all injected platform services.
    #[allow(clippy::too_many_arguments)] // Explicit per-session native service seams stay visible.
    pub fn with_session_components_and_all_services(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access(
            policy,
            credentials,
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

    /// Creates one session with explicit selection-capture and selected-file
    /// text services in addition to the existing native service seams.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage(
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
            TransportUnavailableStorage,
        )
    }

    /// Creates one session with an explicit host-owned application-state store.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage(
        policy: HostPolicy,
        credentials: SessionCredentials,
        ui_document_mailbox: UiDocumentMailbox,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
        storage: impl StorageService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
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
            TransportUnavailableDiagnostics,
        )
    }

    /// Creates one session with an explicit bounded host diagnostics source.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
        policy: HostPolicy,
        credentials: SessionCredentials,
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
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
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
            TransportUnavailableCredentials,
        )
    }

    /// Creates one session with an identity-bound credential service supplied
    /// by the native host. The service owns application identity and target
    /// selection; the pipe peer can supply only a validated local name.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
        policy: HostPolicy,
        credentials: SessionCredentials,
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
    ) -> Self {
        Self {
            decoder: FrameDecoder::new(),
            host: CoreHost::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
                policy,
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
            ),
            ui_document_delivery: UiDocumentDelivery::Legacy(ui_document_mailbox),
            pending_cancellations: BTreeSet::new(),
            state: SessionState::Pending(credentials),
        }
    }
}
