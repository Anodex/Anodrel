//! Public construction paths for the policy-bound core host.
//!
//! Each constructor makes the available host services explicit. Convenience
//! constructors preserve unavailable defaults instead of widening authority.

use super::*;

impl CoreHost {
    pub fn new(policy: HostPolicy) -> Self {
        Self::with_session_components(policy, UiInputMailbox::new(), SessionCloseSignal::default())
    }

    /// Creates a core from one complete host-owned service bundle.
    #[must_use]
    pub fn with_services(policy: HostPolicy, services: HostServices) -> Self {
        Self::with_session_components_and_service_bundle(
            policy,
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            services,
        )
    }

    /// Creates a core from explicit native session components and one complete
    /// host-owned service bundle.
    #[must_use]
    pub fn with_session_components_and_service_bundle(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        services: HostServices,
    ) -> Self {
        Self {
            policy,
            ui_document_session: Some(RefCell::new(UiDocumentSession::new())),
            ui_window_group: None,
            menu_session: RefCell::new(MenuSession::new()),
            ui_input_mailbox: Some(ui_input_mailbox),
            session_close_signal,
            pending_ui_document_update: Some(RefCell::new(None)),
            clipboard: services.clipboard,
            external_links: services.external_links,
            network: services.network,
            notifications: services.notifications,
            window_title: services.window_title,
            window_state: services.window_state,
            window_state_read: services.window_state_read,
            window_focus: services.window_focus,
            window_fullscreen: services.window_fullscreen,
            window_size: services.window_size,
            menu: services.menu,
            ui_fields: services.ui_fields,
            file_dialogs: services.file_dialogs,
            folder_selections: services.folder_selections,
            folder_entries: services.folder_entries,
            file_selections: services.file_selections,
            file_text: services.file_text,
            file_save_selections: services.file_save_selections,
            file_text_write: services.file_text_write,
            file_binary_write: services.file_binary_write,
            storage: services.storage,
            diagnostics: services.diagnostics,
            credentials: services.credentials,
        }
    }

    /// Creates a core whose primary UI state is part of one session-owned
    /// window group.
    ///
    /// The caller constructs the group with the primary view's real document
    /// and input mailboxes before authentication. This core therefore has no
    /// parallel primary document or input state to keep in sync. Existing
    /// targetless UI operations continue to resolve only `main`; Protocol 1.25
    /// adds separate explicit operations for the bounded group without
    /// widening those compatibility paths.
    #[must_use]
    pub fn with_session_window_group_and_service_bundle(
        policy: HostPolicy,
        ui_window_group: UiWindowGroup<WindowTitleProposal>,
        session_close_signal: SessionCloseSignal,
        services: HostServices,
    ) -> Self {
        let mut core = Self::with_session_components_and_service_bundle(
            policy,
            UiInputMailbox::new(),
            session_close_signal,
            services,
        );
        core.ui_document_session = None;
        core.ui_window_group = Some(ui_window_group);
        core.ui_input_mailbox = None;
        core.pending_ui_document_update = None;
        core
    }

    /// Creates a core with only an identity-bound credential service enabled.
    /// Other platform-service seams remain explicitly unavailable.
    pub fn with_credential_service(
        policy: HostPolicy,
        credentials: impl CredentialService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
            policy,
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            UnavailableClipboard,
            UnavailableExternalLinks,
            UnavailableFileDialogs,
            UnavailableFileSelectionService,
            UnavailableFileTextService,
            UnavailableStorage,
            UnavailableDiagnostics,
            credentials,
        )
    }

    /// Creates a host core that validates semantic input from one supplied
    /// per-session mailbox.
    pub fn with_ui_input_mailbox(policy: HostPolicy, ui_input_mailbox: UiInputMailbox) -> Self {
        Self::with_session_components(policy, ui_input_mailbox, SessionCloseSignal::default())
    }

    /// Creates a host core with explicit native input and session-close signals.
    pub fn with_session_components(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
    ) -> Self {
        Self::with_session_components_and_all_services(
            policy,
            ui_input_mailbox,
            session_close_signal,
            UnavailableClipboard,
            UnavailableExternalLinks,
            UnavailableFileDialogs,
        )
    }

    /// Creates a host core with explicit native components and one injected
    /// portable clipboard service.
    pub fn with_session_components_and_clipboard(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services(
            policy,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            UnavailableExternalLinks,
            UnavailableFileDialogs,
        )
    }

    /// Creates a host core with explicit native components and injected
    /// portable clipboard and external-link services.
    pub fn with_session_components_and_services(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services(
            policy,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            UnavailableFileDialogs,
        )
    }

    /// Creates a host core with all currently supported injected platform
    /// services. Dialog implementations must route native UI through the host
    /// UI thread rather than invoking an OS dialog from this core.
    pub fn with_session_components_and_all_services(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access(
            policy,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            UnavailableFileSelectionService,
            UnavailableFileTextService,
        )
    }

    /// Creates a host core with all injected platform services, including the
    /// separate selection-capture and selected-file text boundaries.
    ///
    /// The selection service must bind a picker choice to retained native
    /// identity before returning success. The text service consumes only that
    /// retained state, never a request path.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access(
        policy: HostPolicy,
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

    /// Creates a host core with all injected services, including one
    /// host-selected application-state store for this authenticated session.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage(
        policy: HostPolicy,
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

    /// Creates a host core with the bounded host-owned diagnostic source for
    /// this authenticated session.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
        policy: HostPolicy,
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

    /// Creates a host core with an identity-bound credential service. The
    /// implementation owns the validated application identity and must never
    /// accept a target or identity from the authenticated request.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics_and_credentials(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
        storage: impl StorageService + 'static,
        diagnostics: impl DiagnosticsService + 'static,
        credentials: impl CredentialService + 'static,
    ) -> Self {
        Self {
            policy,
            ui_document_session: Some(RefCell::new(UiDocumentSession::new())),
            ui_window_group: None,
            menu_session: RefCell::new(MenuSession::new()),
            ui_input_mailbox: Some(ui_input_mailbox),
            session_close_signal,
            pending_ui_document_update: Some(RefCell::new(None)),
            clipboard: Box::new(clipboard),
            external_links: Box::new(external_links),
            network: Box::new(UnavailableNetwork),
            // This constructor names each service explicitly and predates
            // notifications. Leaving it unavailable keeps every existing caller
            // exactly as capable as it was, rather than silently widening it.
            notifications: Box::new(UnavailableNotifications),
            window_title: Box::new(UnavailableWindowTitle),
            window_state: Box::new(UnavailableWindowState),
            window_state_read: Box::new(UnavailableWindowStateRead),
            window_focus: Box::new(UnavailableWindowFocus),
            window_fullscreen: Box::new(UnavailableWindowFullscreen),
            window_size: Box::new(UnavailableWindowSize),
            menu: Box::new(UnavailableMenuService),
            ui_fields: Box::new(UnavailableUiFields),
            file_dialogs: Box::new(file_dialogs),
            folder_selections: Box::new(UnavailableFolderSelectionService),
            folder_entries: Box::new(UnavailableFolderEntryService),
            file_selections: Box::new(file_selections),
            file_text: Box::new(file_text),
            file_save_selections: Box::new(UnavailableSaveSelectionService),
            file_text_write: Box::new(UnavailableFileTextWriteService),
            file_binary_write: Box::new(UnavailableFileBinaryWriteService),
            storage: Box::new(storage),
            diagnostics: Box::new(diagnostics),
            credentials: Box::new(credentials),
        }
    }
}
