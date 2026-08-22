#![forbid(unsafe_code)]

//! Policy-bound handling for one native protocol message.
//!
//! Transports authenticate their sessions before calling this module. Incoming
//! capability context is intentionally ignored: only the host-created policy
//! can authorize a privileged operation.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use anodrel_clipboard::{ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText};
use anodrel_credentials::{CredentialName, CredentialService, CredentialServiceError, Secret};
use anodrel_diagnostics::{DiagnosticsService, DiagnosticsServiceError};
use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
use anodrel_file_access::{
    FileBinaryData, FileBinaryDataError, FileBinaryWriteService, FileBinaryWriteServiceError,
    FileSelectionResult, FileSelectionService, FileSelectionServiceError, FileTextService,
    FileTextServiceError, FileTextWriteService, FileTextWriteServiceError, SaveReference,
    SaveSelectionResult, SaveSelectionService, SaveSelectionServiceError, SelectionReference,
    UnavailableFileBinaryWriteService, UnavailableFileSelectionService, UnavailableFileTextService,
    UnavailableFileTextWriteService, UnavailableSaveSelectionService,
};
use anodrel_file_dialog::{
    FileDialogFilter, FileDialogSelection, FileDialogService, FileDialogServiceError,
};
use anodrel_menu::{
    Menu, MenuAction, MenuActionId, MenuModel, MenuService, MenuSession, MenuShortcut, MenuText,
    UnavailableMenuService,
};
use anodrel_network::{NetworkTextService, NetworkTextServiceError, NetworkUrl};
use anodrel_notifications::{
    Notification, NotificationBody, NotificationService, NotificationServiceError,
    NotificationTitle,
};
use anodrel_protocol::{
    Capability, JsonValue, ProtocolErrorCode, ProtocolVersion, RequestEnvelope, ResponseEnvelope,
    is_empty_object, object, sent_at,
};
use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};
use anodrel_ui_session::{
    SessionInteractionCandidate, UiDocumentSession, UiDocumentSnapshot, UiFieldReadError,
    UiFieldReader, UiFieldSnapshot, UiInputMailbox, UiWindowGroup, UiWindowGroupError, UiWindowId,
};
use anodrel_window::{
    WindowFocusService, WindowFocusServiceError, WindowFullscreenMode, WindowFullscreenService,
    WindowFullscreenServiceError, WindowSize, WindowSizeService, WindowSizeServiceError,
    WindowState, WindowStateService, WindowStateServiceError, WindowTitleProposal,
    WindowTitleService, WindowTitleServiceError,
};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_UI_DOCUMENT_REQUEST_BYTES: usize = 24 * 1024;
pub const MAX_CLIPBOARD_TEXT_REQUEST_BYTES: usize = 24 * 1024;
pub const MAX_EXTERNAL_LINK_REQUEST_BYTES: usize = 2 * 1024;
/// Maximum bytes in the exact `network.fetch_text` URL payload.
pub const MAX_NETWORK_FETCH_REQUEST_BYTES: usize = 2 * 1024;
pub const MAX_FILE_DIALOG_REQUEST_BYTES: usize = 2 * 1024;
pub const MAX_FILE_DIALOG_FILTERS: usize = 8;
pub const MAX_FILE_TEXT_RESPONSE_BYTES: usize = 8 * 1024;
pub const MAX_FILE_TEXT_WRITE_BYTES: usize = 8 * 1024;
/// Maximum encoded JSON bytes in one complete native-menu replacement payload.
pub const MAX_MENU_REPLACE_REQUEST_BYTES: usize = 16 * 1024;
pub const MAX_STORAGE_SNAPSHOT_REQUEST_BYTES: usize = 24 * 1024;
const MENU_ACTION_EVENT_SCHEMA_VERSION: ProtocolVersion = ProtocolVersion {
    major: 1,
    minor: 18,
};

/// The exact external UI document format selected by one protocol operation.
///
/// This stays private to the core dispatcher: applications select only an
/// operation name, while document decoding remains in the portable session
/// layer.
#[derive(Clone, Copy)]
enum UiDocumentFormat {
    V1,
    V2,
    V3,
}

impl UiDocumentFormat {
    const fn document_operation(self) -> &'static str {
        match self {
            Self::V1 => "ui.document.replace",
            Self::V2 => "ui.document.replace.v2",
            Self::V3 => "ui.document.replace.v3",
        }
    }

    const fn window_operation(self) -> &'static str {
        match self {
            Self::V1 => "ui.document.replace.window",
            Self::V2 => "ui.document.replace.window.v2",
            Self::V3 => "ui.document.replace.window.v3",
        }
    }

    const fn open_operation(self) -> &'static str {
        match self {
            Self::V1 => "window.open",
            Self::V2 => "window.open.v2",
            Self::V3 => "window.open.v3",
        }
    }
}

/// One host-created, coalescing request to end an authenticated session.
///
/// This value stores no target, payload, callback, or operating-system state.
/// The native host that supplied it decides which resources to close.
#[derive(Clone, Debug, Default)]
pub struct SessionCloseSignal {
    requested: Arc<AtomicBool>,
}

impl SessionCloseSignal {
    /// Records an idempotent request for the host to end its known session.
    pub fn request(&self) {
        self.requested.store(true, Ordering::Release);
    }

    /// Takes one pending close request, if any.
    pub fn take(&self) -> bool {
        self.requested.swap(false, Ordering::AcqRel)
    }
}

#[derive(Clone, Debug)]
pub struct HostPolicy {
    application_id: String,
    granted_capabilities: Vec<Capability>,
    host_name: String,
}

impl HostPolicy {
    pub fn new(
        application_id: impl Into<String>,
        granted_capabilities: Vec<Capability>,
        host_name: impl Into<String>,
    ) -> Result<Self, &'static str> {
        let application_id = application_id.into();
        let host_name = host_name.into();
        if application_id.trim().is_empty() || host_name.trim().is_empty() {
            return Err("application ID and host name must not be empty");
        }
        if granted_capabilities
            .iter()
            .enumerate()
            .any(|(index, capability)| granted_capabilities[..index].contains(capability))
        {
            return Err("host capability grants must not contain duplicates");
        }
        Ok(Self {
            application_id,
            granted_capabilities,
            host_name,
        })
    }

    fn has(&self, capability: Capability) -> bool {
        self.granted_capabilities.contains(&capability)
    }
}

#[derive(Debug)]
pub struct CoreHost {
    policy: HostPolicy,
    ui_document_session: Option<RefCell<UiDocumentSession>>,
    ui_window_group: Option<UiWindowGroup<WindowTitleProposal>>,
    menu_session: RefCell<MenuSession>,
    ui_input_mailbox: Option<UiInputMailbox>,
    session_close_signal: SessionCloseSignal,
    pending_ui_document_update: Option<RefCell<Option<UiDocumentSnapshot>>>,
    clipboard: Box<dyn ClipboardService>,
    external_links: Box<dyn ExternalLinkService>,
    network: Box<dyn NetworkTextService>,
    notifications: Box<dyn NotificationService>,
    window_title: Box<dyn WindowTitleService>,
    window_state: Box<dyn WindowStateService>,
    window_focus: Box<dyn WindowFocusService>,
    window_fullscreen: Box<dyn WindowFullscreenService>,
    window_size: Box<dyn WindowSizeService>,
    menu: Box<dyn MenuService>,
    ui_fields: Box<dyn UiFieldReader>,
    file_dialogs: Box<dyn FileDialogService>,
    file_selections: Box<dyn FileSelectionService>,
    file_text: Box<dyn FileTextService>,
    file_save_selections: Box<dyn SaveSelectionService>,
    file_text_write: Box<dyn FileTextWriteService>,
    file_binary_write: Box<dyn FileBinaryWriteService>,
    storage: Box<dyn StorageService>,
    diagnostics: Box<dyn DiagnosticsService>,
    credentials: Box<dyn CredentialService>,
}

/// Explicit native services owned by one authenticated host session.
///
/// A native composition boundary builds this value from host-validated
/// identity, policy, and native resources before a transport session begins.
/// Protocol messages cannot replace, inspect, or add services after this value
/// has been consumed by [`CoreHost`]. Services are unavailable by default so a
/// newly declared capability never gains ambient operating-system authority.
#[derive(Debug)]
pub struct HostServices {
    clipboard: Box<dyn ClipboardService>,
    external_links: Box<dyn ExternalLinkService>,
    network: Box<dyn NetworkTextService>,
    notifications: Box<dyn NotificationService>,
    window_title: Box<dyn WindowTitleService>,
    window_state: Box<dyn WindowStateService>,
    window_focus: Box<dyn WindowFocusService>,
    window_fullscreen: Box<dyn WindowFullscreenService>,
    window_size: Box<dyn WindowSizeService>,
    menu: Box<dyn MenuService>,
    ui_fields: Box<dyn UiFieldReader>,
    file_dialogs: Box<dyn FileDialogService>,
    file_selections: Box<dyn FileSelectionService>,
    file_text: Box<dyn FileTextService>,
    file_save_selections: Box<dyn SaveSelectionService>,
    file_text_write: Box<dyn FileTextWriteService>,
    file_binary_write: Box<dyn FileBinaryWriteService>,
    storage: Box<dyn StorageService>,
    diagnostics: Box<dyn DiagnosticsService>,
    credentials: Box<dyn CredentialService>,
}

#[derive(Debug)]
struct UnavailableClipboard;

impl ClipboardService for UnavailableClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }

    fn write_text(&self, _text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableExternalLinks;

impl ExternalLinkService for UnavailableExternalLinks {
    fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        Err(ExternalLinkOpenError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableNetwork;

impl NetworkTextService for UnavailableNetwork {
    fn fetch_text(
        &self,
        _url: &NetworkUrl,
    ) -> Result<anodrel_network::NetworkTextResponse, NetworkTextServiceError> {
        Err(NetworkTextServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableNotifications;

impl NotificationService for UnavailableNotifications {
    fn show(&self, _notification: &Notification) -> Result<(), NotificationServiceError> {
        Err(NotificationServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableUiFields;

impl UiFieldReader for UnavailableUiFields {
    fn read(&self) -> Result<UiFieldSnapshot, UiFieldReadError> {
        Err(UiFieldReadError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableWindowTitle;

impl WindowTitleService for UnavailableWindowTitle {
    fn set_title(&self, _proposal: &WindowTitleProposal) -> Result<(), WindowTitleServiceError> {
        Err(WindowTitleServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableWindowState;

impl WindowStateService for UnavailableWindowState {
    fn set_state(&self, _state: WindowState) -> Result<(), WindowStateServiceError> {
        Err(WindowStateServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableWindowFocus;

impl WindowFocusService for UnavailableWindowFocus {
    fn request_focus(&self) -> Result<(), WindowFocusServiceError> {
        Err(WindowFocusServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableWindowFullscreen;

impl WindowFullscreenService for UnavailableWindowFullscreen {
    fn set_fullscreen(
        &self,
        _mode: WindowFullscreenMode,
    ) -> Result<(), WindowFullscreenServiceError> {
        Err(WindowFullscreenServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableWindowSize;

impl WindowSizeService for UnavailableWindowSize {
    fn set_size(&self, _size: WindowSize) -> Result<(), WindowSizeServiceError> {
        Err(WindowSizeServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableFileDialogs;

impl FileDialogService for UnavailableFileDialogs {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableStorage;

impl StorageService for UnavailableStorage {
    fn read(&self) -> Result<StorageRead, StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }

    fn replace(&self, _snapshot: &StorageSnapshot) -> Result<(), StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }

    fn clear(&self) -> Result<(), StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableDiagnostics;

impl DiagnosticsService for UnavailableDiagnostics {
    fn entries(&self) -> Result<Vec<anodrel_diagnostics::Entry>, DiagnosticsServiceError> {
        Err(DiagnosticsServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableCredentials;

impl CredentialService for UnavailableCredentials {
    fn read(&self, _name: &CredentialName) -> Result<Secret, CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }

    fn write(
        &self,
        _name: &CredentialName,
        _secret: &Secret,
    ) -> Result<(), CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }

    fn delete(&self, _name: &CredentialName) -> Result<bool, CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }
}

impl HostServices {
    /// Creates a service bundle with every platform service explicitly
    /// unavailable.
    #[must_use]
    pub fn unavailable() -> Self {
        Self {
            clipboard: Box::new(UnavailableClipboard),
            external_links: Box::new(UnavailableExternalLinks),
            network: Box::new(UnavailableNetwork),
            notifications: Box::new(UnavailableNotifications),
            window_title: Box::new(UnavailableWindowTitle),
            window_state: Box::new(UnavailableWindowState),
            window_focus: Box::new(UnavailableWindowFocus),
            window_fullscreen: Box::new(UnavailableWindowFullscreen),
            window_size: Box::new(UnavailableWindowSize),
            menu: Box::new(UnavailableMenuService),
            ui_fields: Box::new(UnavailableUiFields),
            file_dialogs: Box::new(UnavailableFileDialogs),
            file_selections: Box::new(UnavailableFileSelectionService),
            file_text: Box::new(UnavailableFileTextService),
            file_save_selections: Box::new(UnavailableSaveSelectionService),
            file_text_write: Box::new(UnavailableFileTextWriteService),
            file_binary_write: Box::new(UnavailableFileBinaryWriteService),
            storage: Box::new(UnavailableStorage),
            diagnostics: Box::new(UnavailableDiagnostics),
            credentials: Box::new(UnavailableCredentials),
        }
    }

    /// Replaces the session's bounded text clipboard service.
    #[must_use]
    pub fn with_clipboard(mut self, service: impl ClipboardService + 'static) -> Self {
        self.clipboard = Box::new(service);
        self
    }

    /// Replaces the session's validated external-link service.
    #[must_use]
    pub fn with_external_links(mut self, service: impl ExternalLinkService + 'static) -> Self {
        self.external_links = Box::new(service);
        self
    }

    /// Replaces the session's host-authorized HTTPS text-fetch service.
    ///
    /// The supplied service owns its exact origin policy and every native
    /// network decision. It receives only a previously validated URL and
    /// exposes no headers, cookies, credentials, or native handles.
    #[must_use]
    pub fn with_network(mut self, service: impl NetworkTextService + 'static) -> Self {
        self.network = Box::new(service);
        self
    }

    /// Replaces the session's host-routed notification service.
    ///
    /// The supplied service must reach the operating system through the host UI
    /// thread. A session worker never calls Shell32 itself.
    #[must_use]
    pub fn with_notifications(mut self, service: impl NotificationService + 'static) -> Self {
        self.notifications = Box::new(service);
        self
    }

    /// Replaces the session's field-value reader.
    ///
    /// The supplied reader must answer only for this session's own current
    /// surface, and must accept no selector — see `docs/UI_FIELDS.md` for why
    /// that absence is the security property rather than a simplification.
    #[must_use]
    pub fn with_ui_fields(mut self, reader: impl UiFieldReader + 'static) -> Self {
        self.ui_fields = Box::new(reader);
        self
    }

    /// Replaces the session's host-routed window-title service.
    ///
    /// The supplied service must reach User32 through the host UI thread, and
    /// must apply the title only to the window this session owns. A session
    /// worker never calls User32 itself, and never names a window.
    #[must_use]
    pub fn with_window_title(mut self, service: impl WindowTitleService + 'static) -> Self {
        self.window_title = Box::new(service);
        self
    }

    /// Replaces the session's host-routed window-state service.
    ///
    /// The supplied service must apply only the closed state to the requesting
    /// session's own window from the host UI thread. It accepts neither a
    /// target nor a native command value; see `docs/WINDOW_STATE.md`.
    #[must_use]
    pub fn with_window_state(mut self, service: impl WindowStateService + 'static) -> Self {
        self.window_state = Box::new(service);
        self
    }

    /// Replaces the session's host-routed window-focus service.
    ///
    /// The supplied service must ask Windows to foreground only the requesting
    /// session's own window from the host UI thread. It accepts no target and
    /// exposes no foreground or activation state; see `docs/WINDOW_FOCUS.md`.
    #[must_use]
    pub fn with_window_focus(mut self, service: impl WindowFocusService + 'static) -> Self {
        self.window_focus = Box::new(service);
        self
    }

    /// Replaces the session's host-routed fullscreen service.
    ///
    /// The supplied service must apply only the closed reversible mode to the
    /// requesting session's own window from the host UI thread. It accepts no
    /// target, monitor, geometry, display mode, or state query; see
    /// `docs/WINDOW_FULLSCREEN.md`.
    #[must_use]
    pub fn with_window_fullscreen(
        mut self,
        service: impl WindowFullscreenService + 'static,
    ) -> Self {
        self.window_fullscreen = Box::new(service);
        self
    }

    /// Replaces the session's host-routed client-size service.
    ///
    /// The supplied service must apply only bounded logical client dimensions
    /// to the requesting session's own window from the host UI thread. It
    /// accepts no target, position, monitor, or geometry query; see
    /// `docs/WINDOW_SIZE.md`.
    #[must_use]
    pub fn with_window_size(mut self, service: impl WindowSizeService + 'static) -> Self {
        self.window_size = Box::new(service);
        self
    }

    /// Replaces the session's host-routed native-menu service.
    ///
    /// The service owns all native command identifiers and must route every
    /// operation through this session's UI thread. It receives only a complete
    /// validated semantic model and its host revision.
    #[must_use]
    pub fn with_menu(mut self, service: impl MenuService + 'static) -> Self {
        self.menu = Box::new(service);
        self
    }

    /// Replaces the session's host-routed file-dialog service.
    #[must_use]
    pub fn with_file_dialogs(mut self, service: impl FileDialogService + 'static) -> Self {
        self.file_dialogs = Box::new(service);
        self
    }

    /// Replaces the session's retained file-selection service.
    #[must_use]
    pub fn with_file_selections(mut self, service: impl FileSelectionService + 'static) -> Self {
        self.file_selections = Box::new(service);
        self
    }

    /// Replaces the session's selected-file text service.
    #[must_use]
    pub fn with_file_text(mut self, service: impl FileTextService + 'static) -> Self {
        self.file_text = Box::new(service);
        self
    }

    /// Replaces the session's retained selected-output service.
    #[must_use]
    pub fn with_file_save_selections(
        mut self,
        service: impl SaveSelectionService + 'static,
    ) -> Self {
        self.file_save_selections = Box::new(service);
        self
    }

    /// Replaces the session's selected-output text writer.
    #[must_use]
    pub fn with_file_text_write(mut self, service: impl FileTextWriteService + 'static) -> Self {
        self.file_text_write = Box::new(service);
        self
    }

    /// Replaces the session's selected-output binary writer.
    ///
    /// The supplied service receives only already-decoded bounded bytes and a
    /// retained save reference. It must never reopen a path or decode a
    /// protocol value; see `docs/FILE_BINARY_WRITE.md`.
    #[must_use]
    pub fn with_file_binary_write(
        mut self,
        service: impl FileBinaryWriteService + 'static,
    ) -> Self {
        self.file_binary_write = Box::new(service);
        self
    }

    /// Replaces the session's host-owned application-state service.
    #[must_use]
    pub fn with_storage(mut self, service: impl StorageService + 'static) -> Self {
        self.storage = Box::new(service);
        self
    }

    /// Replaces the session's closed host diagnostics source.
    #[must_use]
    pub fn with_diagnostics(mut self, service: impl DiagnosticsService + 'static) -> Self {
        self.diagnostics = Box::new(service);
        self
    }

    /// Replaces the session's identity-bound credential service.
    #[must_use]
    pub fn with_credentials(mut self, service: impl CredentialService + 'static) -> Self {
        self.credentials = Box::new(service);
        self
    }
}

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
            window_focus: services.window_focus,
            window_fullscreen: services.window_fullscreen,
            window_size: services.window_size,
            menu: services.menu,
            ui_fields: services.ui_fields,
            file_dialogs: services.file_dialogs,
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
            window_focus: Box::new(UnavailableWindowFocus),
            window_fullscreen: Box::new(UnavailableWindowFullscreen),
            window_size: Box::new(UnavailableWindowSize),
            menu: Box::new(UnavailableMenuService),
            ui_fields: Box::new(UnavailableUiFields),
            file_dialogs: Box::new(file_dialogs),
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

    /// Takes the latest accepted document snapshot not yet observed by the
    /// transport that owns this core host.
    pub fn take_ui_document_update(&self) -> Option<UiDocumentSnapshot> {
        self.pending_ui_document_update
            .as_ref()
            .and_then(|update| update.borrow_mut().take())
    }

    pub fn handle_json(&self, message: &str) -> String {
        let response = if message.len() > MAX_REQUEST_BYTES {
            self.failure(
                "invalid-request".to_owned(),
                ProtocolErrorCode::RequestInvalid,
                "Request exceeded the native transport message-size limit.",
                None,
            )
        } else {
            match JsonValue::parse(message) {
                Ok(value) => match RequestEnvelope::from_json(value) {
                    Ok(request) => self.handle(request),
                    Err(_) => self.failure(
                        "invalid-request".to_owned(),
                        ProtocolErrorCode::RequestInvalid,
                        "Request envelope is malformed.",
                        None,
                    ),
                },
                Err(_) => self.failure(
                    "invalid-request".to_owned(),
                    ProtocolErrorCode::RequestInvalid,
                    "Request envelope is malformed.",
                    None,
                ),
            }
        };
        response.to_json()
    }

    /// Produces the safe result for a request whose cancellation was observed
    /// by the authenticated transport before this core began processing it.
    ///
    /// The transport obtains `request_id` only from a validated request
    /// envelope. This method does not retain cancellation state or attempt to
    /// roll back work that has already entered an operation handler.
    pub fn cancelled_response(&self, request_id: String) -> String {
        self.failure(
            request_id,
            ProtocolErrorCode::RequestCancelled,
            "Request was cancelled before the host began processing it.",
            None,
        )
        .to_json()
    }

    fn handle(&self, request: RequestEnvelope) -> JsonValue {
        if !request.protocol_version.is_supported() {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::ProtocolVersionUnsupported,
                format!(
                    "Protocol {}.{} is not supported.",
                    request.protocol_version.major, request.protocol_version.minor
                ),
                None,
            );
        }

        match request.operation.as_str() {
            "platform.ping" => self.handle_ping(request),
            "platform.capabilities" => self.handle_capabilities(request),
            "platform.health" => self.handle_health(request),
            "diagnostics.entries.read" if request.protocol_version.minor >= 11 => {
                self.handle_diagnostics_entries_read(request)
            }
            "credential.read" if request.protocol_version.minor >= 12 => {
                self.handle_credential_read(request)
            }
            "credential.write" if request.protocol_version.minor >= 12 => {
                self.handle_credential_write(request)
            }
            "credential.delete" if request.protocol_version.minor >= 12 => {
                self.handle_credential_delete(request)
            }
            "notification.show" if request.protocol_version.minor >= 13 => {
                self.handle_notification_show(request)
            }
            "window.title.set" if request.protocol_version.minor >= 14 => {
                self.handle_window_title_set(request)
            }
            "ui.fields.read" if request.protocol_version.minor >= 15 => {
                self.handle_ui_fields_read(request)
            }
            "window.state.set" if request.protocol_version.minor >= 16 => {
                self.handle_window_state_set(request)
            }
            "window.focus.request" if request.protocol_version.minor >= 20 => {
                self.handle_window_focus_request(request)
            }
            "window.fullscreen.set" if request.protocol_version.minor >= 21 => {
                self.handle_window_fullscreen_set(request)
            }
            "window.size.set" if request.protocol_version.minor >= 23 => {
                self.handle_window_size_set(request)
            }
            "window.open" if request.protocol_version.minor >= 25 => {
                self.handle_window_open(request, UiDocumentFormat::V1)
            }
            "window.open.v3" if request.protocol_version.minor >= 26 => {
                self.handle_window_open(request, UiDocumentFormat::V3)
            }
            "window.close" if request.protocol_version.minor >= 25 => {
                self.handle_window_close(request)
            }
            "menu.replace" if request.protocol_version.minor >= 18 => {
                self.handle_menu_replace(request)
            }
            "ui.document.replace" if request.protocol_version.minor >= 1 => {
                self.handle_ui_document_replace(request, UiDocumentFormat::V1)
            }
            "ui.document.replace.v2" if request.protocol_version.minor >= 4 => {
                self.handle_ui_document_replace(request, UiDocumentFormat::V2)
            }
            "ui.document.replace.v3" if request.protocol_version.minor >= 26 => {
                self.handle_ui_document_replace(request, UiDocumentFormat::V3)
            }
            "ui.document.replace.window" if request.protocol_version.minor >= 25 => {
                self.handle_ui_document_replace_window(request, UiDocumentFormat::V1)
            }
            "ui.document.replace.window.v3" if request.protocol_version.minor >= 26 => {
                self.handle_ui_document_replace_window(request, UiDocumentFormat::V3)
            }
            "ui.events.read" if request.protocol_version.minor >= 2 => {
                self.handle_ui_events_read(request)
            }
            "ui.events.read.window" if request.protocol_version.minor >= 25 => {
                self.handle_ui_events_read_window(request)
            }
            "session.close" if request.protocol_version.minor >= 3 => {
                self.handle_session_close(request)
            }
            "clipboard.read" if request.protocol_version.minor >= 5 => {
                self.handle_clipboard_read(request)
            }
            "clipboard.write" if request.protocol_version.minor >= 5 => {
                self.handle_clipboard_write(request)
            }
            "external.open" if request.protocol_version.minor >= 6 => {
                self.handle_external_open(request)
            }
            "network.fetch_text" if request.protocol_version.minor >= 19 => {
                self.handle_network_fetch_text(request)
            }
            "dialog.open_file" if request.protocol_version.minor >= 7 => {
                self.handle_file_dialog_open(request)
            }
            "dialog.save_file" if request.protocol_version.minor >= 8 => {
                self.handle_file_dialog_save(request)
            }
            "dialog.open_file.v2" if request.protocol_version.minor >= 9 => {
                self.handle_file_dialog_open_with_reference(request)
            }
            "file.read_text" if request.protocol_version.minor >= 9 => {
                self.handle_file_text_read(request)
            }
            "dialog.save_file.v2" if request.protocol_version.minor >= 17 => {
                self.handle_file_dialog_save_with_reference(request)
            }
            "file.write_text" if request.protocol_version.minor >= 17 => {
                self.handle_file_text_write(request)
            }
            "file.write_binary" if request.protocol_version.minor >= 22 => {
                self.handle_file_binary_write(request)
            }
            "storage.state.read" if request.protocol_version.minor >= 10 => {
                self.handle_storage_read(request)
            }
            "storage.state.replace" if request.protocol_version.minor >= 10 => {
                self.handle_storage_replace(request)
            }
            "storage.state.clear" if request.protocol_version.minor >= 10 => {
                self.handle_storage_clear(request)
            }
            _ => self.failure(
                request.request_id,
                ProtocolErrorCode::OperationUnsupported,
                format!(
                    "Operation {} is not supported by this host.",
                    request.operation
                ),
                None,
            ),
        }
    }

    fn handle_ping(&self, request: RequestEnvelope) -> JsonValue {
        if sent_at(&request.payload).is_none() {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "platform.ping requires a sentAt string.",
                None,
            );
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                ("receivedAt", JsonValue::String(rfc3339_now())),
                ("hostName", JsonValue::String(self.policy.host_name.clone())),
            ]),
        )
    }

    fn handle_capabilities(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "platform.capabilities does not accept a payload.",
                None,
            );
        }
        let capabilities = self
            .policy
            .granted_capabilities
            .iter()
            .map(|capability| JsonValue::String(capability.as_str().to_owned()))
            .collect();
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                (
                    "applicationId",
                    JsonValue::String(self.policy.application_id.clone()),
                ),
                ("grantedCapabilities", JsonValue::Array(capabilities)),
            ]),
        )
    }

    fn handle_health(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "platform.health does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::DiagnosticsRead) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                "platform.health requires the diagnostics.read capability.",
                Some(BTreeMap::from([(
                    "capability".to_owned(),
                    JsonValue::String("diagnostics.read".to_owned()),
                )])),
            );
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                ("status", JsonValue::String("ready".to_owned())),
                ("hostName", JsonValue::String(self.policy.host_name.clone())),
                ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
            ]),
        )
    }

    fn handle_diagnostics_entries_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "diagnostics.entries.read does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::DiagnosticsRead) {
            return self.capability_denied(request.request_id, "diagnostics.read");
        }
        match self.diagnostics.entries() {
            Ok(entries) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([(
                    "entries",
                    JsonValue::Array(
                        entries
                            .into_iter()
                            .map(|entry| {
                                object([
                                    ("sequence", JsonValue::String(entry.sequence().to_string())),
                                    ("level", JsonValue::String(entry.level().label().to_owned())),
                                    ("component", JsonValue::String(entry.component().to_owned())),
                                    ("event", JsonValue::String(entry.message().to_owned())),
                                ])
                            })
                            .collect(),
                    ),
                )]),
            ),
            Err(DiagnosticsServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DiagnosticsUnavailable,
                "diagnostic entries are unavailable.",
                None,
            ),
        }
    }

    fn handle_ui_document_replace(
        &self,
        request: RequestEnvelope,
        format: UiDocumentFormat,
    ) -> JsonValue {
        let operation = format.document_operation();
        let Some(document) = ui_document_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} requires one document string."),
                None,
            );
        };
        if !self.policy.has(Capability::UiDocumentWrite) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                format!("{operation} requires the ui.document.write capability."),
                Some(BTreeMap::from([(
                    "capability".to_owned(),
                    JsonValue::String("ui.document.write".to_owned()),
                )])),
            );
        }
        if document.len() > MAX_UI_DOCUMENT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document exceeded the operation size limit."),
                None,
            );
        }

        let snapshot = if let Some(group) = &self.ui_window_group {
            let primary = UiWindowId::primary();
            let replacement = match format {
                UiDocumentFormat::V1 => group.replace_document(&primary, document),
                UiDocumentFormat::V2 => group.replace_document_v2(&primary, document),
                UiDocumentFormat::V3 => group.replace_document_v3(&primary, document),
            };
            replacement.ok().map(|snapshot| snapshot.snapshot().clone())
        } else {
            let session = self
                .ui_document_session
                .as_ref()
                .expect("legacy core has a primary document session");
            let mut session = session.borrow_mut();
            let revision = match format {
                UiDocumentFormat::V1 => session.replace_document(document),
                UiDocumentFormat::V2 => session.replace_document_v2(document),
                UiDocumentFormat::V3 => session.replace_document_v3(document),
            };
            revision.ok().and_then(|revision| {
                session
                    .snapshot()
                    .filter(|snapshot| snapshot.revision() == revision)
            })
        };
        let Some(snapshot) = snapshot else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document is invalid."),
                None,
            );
        };
        let revision = snapshot.revision();
        if let Some(update) = &self.pending_ui_document_update {
            *update.borrow_mut() = Some(snapshot);
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([("revision", JsonValue::String(revision.value().to_string()))]),
        )
    }

    /// Opens one bounded secondary view in this authenticated session group.
    ///
    /// The worker owns no native window. It can wait only for the portable
    /// group to report that the host UI thread created and registered one
    /// private native view. A successful opaque ID is therefore never a
    /// speculative reservation or a native handle.
    fn handle_window_open(&self, request: RequestEnvelope, format: UiDocumentFormat) -> JsonValue {
        let operation = format.open_operation();
        let Some((title, document)) = window_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} requires one title and one document string."),
                None,
            );
        };
        if !self.policy.has(Capability::WindowOpen) {
            return self.capability_denied(request.request_id, operation);
        }
        if !self.policy.has(Capability::UiDocumentWrite) {
            return self.capability_denied(request.request_id, "ui.document.write");
        }
        if document.len() > MAX_UI_DOCUMENT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document exceeded the operation size limit."),
                None,
            );
        }
        let Ok(title) = WindowTitleProposal::new(title) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowTitleInvalid,
                format!("{operation} title is invalid."),
                None,
            );
        };
        let Some(group) = &self.ui_window_group else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            );
        };
        let opened = match format {
            UiDocumentFormat::V1 => group.open_secondary(title, document),
            UiDocumentFormat::V3 => group.open_secondary_v3(title, document),
            UiDocumentFormat::V2 => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::OperationUnsupported,
                    "window.open.v2 is not implemented.",
                    None,
                );
            }
        };
        match opened {
            Ok(id) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("windowId", JsonValue::String(id.to_protocol_string()))]),
            ),
            Err(UiWindowGroupError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "another session window creation is pending.",
                None,
            ),
            Err(UiWindowGroupError::DocumentRejected(_)) => self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document is invalid."),
                None,
            ),
            Err(UiWindowGroupError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            ),
        }
    }

    /// Requests a host-owned close for one current secondary view.
    ///
    /// The protocol acknowledges queueing only. Windows may still be
    /// processing the request, and the actual logical view remains available
    /// until the native destroy path removes its private mapping.
    fn handle_window_close(&self, request: RequestEnvelope) -> JsonValue {
        let Some(id) = secondary_window_id_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.close requires one canonical secondary windowId.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowClose) {
            return self.capability_denied(request.request_id, "window.close");
        }
        let Some(group) = &self.ui_window_group else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            );
        };
        if group.request_secondary_close(&id).is_err() {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the requested session window is unavailable.",
                None,
            );
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([("status", JsonValue::String("requested".to_owned()))]),
        )
    }

    /// Replaces the strict v1 document of one logical session view.
    ///
    /// `main` remains a legal target here so callers can keep one uniform
    /// document-update path after opening a secondary. Closing `main` remains
    /// forbidden: it is the group anchor rather than an ordinary target.
    fn handle_ui_document_replace_window(
        &self,
        request: RequestEnvelope,
        format: UiDocumentFormat,
    ) -> JsonValue {
        let operation = format.window_operation();
        let Some((id, document)) = window_document_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} requires one canonical windowId and document string."),
                None,
            );
        };
        if !self.policy.has(Capability::UiDocumentWrite) {
            return self.capability_denied(request.request_id, "ui.document.write");
        }
        if document.len() > MAX_UI_DOCUMENT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document exceeded the operation size limit."),
                None,
            );
        }
        let Some(group) = &self.ui_window_group else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            );
        };
        let replacement = match format {
            UiDocumentFormat::V1 => group.replace_document(&id, document),
            UiDocumentFormat::V3 => group.replace_document_v3(&id, document),
            UiDocumentFormat::V2 => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::OperationUnsupported,
                    "ui.document.replace.window.v2 is not implemented.",
                    None,
                );
            }
        };
        match replacement {
            Ok(snapshot) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([(
                    "revision",
                    JsonValue::String(snapshot.snapshot().revision().value().to_string()),
                )]),
            ),
            Err(UiWindowGroupError::DocumentRejected(_)) => self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document is invalid."),
                None,
            ),
            Err(UiWindowGroupError::Busy | UiWindowGroupError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the requested session window is unavailable.",
                None,
            ),
        }
    }

    fn handle_menu_replace(&self, request: RequestEnvelope) -> JsonValue {
        let Some(model) =
            menu_replace_payload(&request.payload, request.protocol_version.minor >= 24)
        else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "menu.replace requires one exact bounded complete menu model.",
                None,
            );
        };
        if !self.policy.has(Capability::MenuWrite) {
            return self.capability_denied(request.request_id, "menu.write");
        }

        let revision = match self.menu_session.borrow().next_revision() {
            Ok(revision) => revision,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::MenuUnavailable,
                    "the session menu is unavailable.",
                    None,
                );
            }
        };
        if self.menu.replace(revision, model.clone()).is_err() {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::MenuUnavailable,
                "the session menu is unavailable.",
                None,
            );
        }
        match self.menu_session.borrow_mut().replace(model) {
            Ok(committed_revision) => {
                debug_assert_eq!(committed_revision, revision);
                ResponseEnvelope::success(
                    request.request_id,
                    &self.policy.host_name,
                    object([(
                        "revision",
                        JsonValue::String(committed_revision.value().to_string()),
                    )]),
                )
            }
            Err(_) => self.failure(
                request.request_id,
                ProtocolErrorCode::MenuUnavailable,
                "the session menu is unavailable.",
                None,
            ),
        }
    }

    fn handle_ui_events_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "ui.events.read does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::UiEventsRead) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                "ui.events.read requires the ui.events.read capability.",
                Some(BTreeMap::from([(
                    "capability".to_owned(),
                    JsonValue::String("ui.events.read".to_owned()),
                )])),
            );
        }

        let batch = if let Some(group) = &self.ui_window_group {
            match group.drain_input_batch(&UiWindowId::primary()) {
                Ok(batch) => batch,
                Err(_) => {
                    return self.failure(
                        request.request_id,
                        ProtocolErrorCode::WindowUnavailable,
                        "the session UI view is unavailable.",
                        None,
                    );
                }
            }
        } else {
            self.ui_input_mailbox
                .as_ref()
                .expect("legacy core has a primary input mailbox")
                .drain()
        };
        let dropped = batch.dropped();
        let mut discarded = 0_u32;
        let mut events = Vec::new();
        for candidate in batch.into_candidates() {
            match candidate {
                SessionInteractionCandidate::Ui(candidate) => {
                    let (revision, event) = candidate.into_parts();
                    let accepted = if let Some(group) = &self.ui_window_group {
                        group
                            .accept_event(&UiWindowId::primary(), revision, event)
                            .ok()
                    } else {
                        self.ui_document_session
                            .as_ref()
                            .expect("legacy core has a primary document session")
                            .borrow()
                            .accept_event(revision, event)
                            .ok()
                    };
                    match accepted {
                        Some(event) => events.push(ui_action_event(event)),
                        None => discarded = discarded.saturating_add(1),
                    }
                }
                SessionInteractionCandidate::Menu(candidate) => {
                    let (revision, action) = candidate.into_parts();
                    match self.menu_session.borrow().accept_action(revision, action) {
                        Ok(event) => events.push(menu_action_event(event)),
                        Err(_) => discarded = discarded.saturating_add(1),
                    }
                }
            }
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                ("events", JsonValue::Array(events)),
                ("dropped", JsonValue::Number(dropped.to_string())),
                ("discarded", JsonValue::Number(discarded.to_string())),
            ]),
        )
    }

    /// Drains bounded semantic input from every current view in this session.
    ///
    /// Batches retain their own view-local input order. The group deliberately
    /// makes no cross-view timing claim, even though its private iteration is
    /// deterministic for testability. Every accepted event receives an opaque
    /// `windowId` tag so application code can validate it against the view
    /// identity it created without learning any native state.
    fn handle_ui_events_read_window(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "ui.events.read.window does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::UiEventsRead) {
            return self.capability_denied(request.request_id, "ui.events.read");
        }
        let Some(group) = &self.ui_window_group else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            );
        };

        let mut dropped = 0_u32;
        let mut discarded = 0_u32;
        let mut events = Vec::new();
        for window_batch in group.drain_input_batches() {
            let (id, batch) = window_batch.into_parts();
            dropped = dropped.saturating_add(batch.dropped());
            for candidate in batch.into_candidates() {
                match candidate {
                    SessionInteractionCandidate::Ui(candidate) => {
                        let (revision, event) = candidate.into_parts();
                        match group.accept_event(&id, revision, event) {
                            Ok(event) => events.push(window_ui_action_event(&id, event)),
                            Err(_) => discarded = discarded.saturating_add(1),
                        }
                    }
                    SessionInteractionCandidate::Menu(candidate) if id.is_primary() => {
                        let (revision, action) = candidate.into_parts();
                        match self.menu_session.borrow().accept_action(revision, action) {
                            Ok(event) => events.push(window_menu_action_event(&id, event)),
                            Err(_) => discarded = discarded.saturating_add(1),
                        }
                    }
                    SessionInteractionCandidate::Menu(_) => {
                        // A secondary receives no menu bridge. If a malformed
                        // host route ever places one there, fail it closed.
                        discarded = discarded.saturating_add(1);
                    }
                }
            }
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                ("events", JsonValue::Array(events)),
                ("dropped", JsonValue::Number(dropped.to_string())),
                ("discarded", JsonValue::Number(discarded.to_string())),
            ]),
        )
    }

    fn handle_session_close(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "session.close does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::SessionClose) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                "session.close requires the session.close capability.",
                Some(BTreeMap::from([(
                    "capability".to_owned(),
                    JsonValue::String("session.close".to_owned()),
                )])),
            );
        }
        self.session_close_signal.request();
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([("status", JsonValue::String("accepted".to_owned()))]),
        )
    }

    fn handle_clipboard_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "clipboard.read does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::ClipboardRead) {
            return self.capability_denied(request.request_id, "clipboard.read");
        }
        match self.clipboard.read_text() {
            Ok(ClipboardRead::Text(text)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("text".to_owned())),
                    ("text", JsonValue::String(text.as_str().to_owned())),
                ]),
            ),
            Ok(ClipboardRead::NoText) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("no_text".to_owned()))]),
            ),
            Err(error) => self.clipboard_failure(request.request_id, error),
        }
    }

    fn handle_clipboard_write(&self, request: RequestEnvelope) -> JsonValue {
        let Some(text) = clipboard_write_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "clipboard.write requires one bounded text string.",
                None,
            );
        };
        if text.len() > MAX_CLIPBOARD_TEXT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "clipboard.write text exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::ClipboardWrite) {
            return self.capability_denied(request.request_id, "clipboard.write");
        }
        let text = match ClipboardText::new(text) {
            Ok(text) => text,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "clipboard.write text exceeded the portable size limit.",
                    None,
                );
            }
        };
        match self.clipboard.write_text(&text) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("written".to_owned()))]),
            ),
            Err(error) => self.clipboard_failure(request.request_id, error),
        }
    }

    fn handle_external_open(&self, request: RequestEnvelope) -> JsonValue {
        let Some(url) = external_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "external.open requires one bounded URL string.",
                None,
            );
        };
        if url.len() > MAX_EXTERNAL_LINK_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "external.open URL exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::ExternalOpen) {
            return self.capability_denied(request.request_id, "external.open");
        }
        let link = match ExternalLink::parse(url) {
            Ok(link) => link,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "external.open URL is invalid.",
                    None,
                );
            }
        };
        match self.external_links.open(&link) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("opened".to_owned()))]),
            ),
            Err(ExternalLinkOpenError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::ExternalUnavailable,
                "external link handler is unavailable.",
                None,
            ),
        }
    }

    /// Performs one host-authorized, bounded HTTPS text fetch.
    ///
    /// The core validates only the protocol URL and grant. The injected native
    /// service owns exact-origin policy and returns only public-safe result
    /// categories, so this layer cannot expose network diagnostics.
    fn handle_network_fetch_text(&self, request: RequestEnvelope) -> JsonValue {
        let Some(url) = network_fetch_text_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "network.fetch_text requires one bounded URL string.",
                None,
            );
        };
        if url.len() > MAX_NETWORK_FETCH_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "network.fetch_text URL exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::NetworkFetch) {
            return self.capability_denied(request.request_id, "network.fetch");
        }
        let url = match NetworkUrl::parse(url) {
            Ok(url) => url,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "network.fetch_text URL is invalid.",
                    None,
                );
            }
        };
        match self.network.fetch_text(&url) {
            Ok(response) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    (
                        "statusCode",
                        JsonValue::Number(response.status_code().to_string()),
                    ),
                    ("text", JsonValue::String(response.text().to_owned())),
                ]),
            ),
            Err(NetworkTextServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::NetworkUnavailable,
                "network text fetch is unavailable.",
                None,
            ),
            Err(NetworkTextServiceError::ResponseInvalid) => self.failure(
                request.request_id,
                ProtocolErrorCode::NetworkResponseInvalid,
                "network text response is invalid.",
                None,
            ),
        }
    }

    /// Shows one bounded notification for an authenticated session.
    ///
    /// The result reports only that the host handed the values over. It must
    /// never describe what the user experienced.
    fn handle_notification_show(&self, request: RequestEnvelope) -> JsonValue {
        let Some((title, body)) = notification_show_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "notification.show requires one title and one body string.",
                None,
            );
        };
        if !self.policy.has(Capability::NotificationShow) {
            return self.capability_denied(request.request_id, "notification.show");
        }

        // Validation failures never echo the offending text back: a rejected
        // notification must not become a way to have the host repeat content.
        let (Ok(title), Ok(body)) = (NotificationTitle::new(title), NotificationBody::new(body))
        else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::NotificationTextInvalid,
                "notification.show text is invalid.",
                None,
            );
        };

        match self.notifications.show(&Notification::new(title, body)) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("shown".to_owned()))]),
            ),
            Err(NotificationServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::NotificationUnavailable,
                "notifications are unavailable.",
                None,
            ),
            Err(NotificationServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::NotificationBusy,
                "a notification is already pending.",
                None,
            ),
        }
    }

    /// Proposes the title of this session's own window.
    ///
    /// The request names no window. The host resolves it from the authenticated
    /// session, and the service composes the displayed caption with a validated
    /// application-name suffix the proposal cannot suppress or forge. Success
    /// reports acceptance only: returning the composed caption would hand the
    /// application a way to probe the host's framing, and it already knows both
    /// halves. See `docs/WINDOW_TITLE.md` and Decision 0066.
    fn handle_window_title_set(&self, request: RequestEnvelope) -> JsonValue {
        let Some(title) = window_title_set_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.title.set requires one title string.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowTitle) {
            return self.capability_denied(request.request_id, "window.title.set");
        }

        // A rejected proposal must not become a way to have the host repeat
        // text back: the failure names the rule, never the value.
        let Ok(proposal) = WindowTitleProposal::new(title) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowTitleInvalid,
                "window.title.set title is invalid.",
                None,
            );
        };

        match self.window_title.set_title(&proposal) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("applied".to_owned()))]),
            ),
            Err(WindowTitleServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no window is available to title.",
                None,
            ),
            Err(WindowTitleServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window title change is already pending.",
                None,
            ),
        }
    }

    /// Applies one closed presentation state to this session's own window.
    ///
    /// There is no target and no state readback. The worker gives the portable
    /// enum to a service that must route it to the owning UI thread; success is
    /// acceptance only, never an observation about the native window.
    fn handle_window_state_set(&self, request: RequestEnvelope) -> JsonValue {
        let Some(state) = window_state_set_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.state.set requires one closed state string.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowState) {
            return self.capability_denied(request.request_id, "window.state.set");
        }

        match self.window_state.set_state(state) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("applied".to_owned()))]),
            ),
            Err(WindowStateServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no session window is available for the requested state.",
                None,
            ),
            Err(WindowStateServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window state change is already pending.",
                None,
            ),
        }
    }

    /// Applies one reversible fullscreen mode to this session's own window.
    ///
    /// The payload is one closed mode rather than a monitor, rectangle,
    /// display-mode, style, or native command. The host resolves the window
    /// from the authenticated session and retains all restoration facts; a
    /// success response is action acceptance, not fullscreen-state readback.
    fn handle_window_fullscreen_set(&self, request: RequestEnvelope) -> JsonValue {
        let Some(mode) = window_fullscreen_set_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.fullscreen.set requires one closed mode string.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowFullscreen) {
            return self.capability_denied(request.request_id, "window.fullscreen.set");
        }

        match self.window_fullscreen.set_fullscreen(mode) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("applied".to_owned()))]),
            ),
            Err(WindowFullscreenServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no session window is available for the requested fullscreen mode.",
                None,
            ),
            Err(WindowFullscreenServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window fullscreen change is already pending.",
                None,
            ),
        }
    }

    /// Requests one bounded logical client size for this session's own window.
    ///
    /// The request carries neither a native rectangle nor a target. The host
    /// service resolves the one session window, converts the logical client
    /// dimensions at its current DPI, and returns acceptance only. No response
    /// becomes geometry, monitor, DPI, or presentation-state readback.
    fn handle_window_size_set(&self, request: RequestEnvelope) -> JsonValue {
        let Some(size) = window_size_set_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.size.set requires one bounded logical width and height.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowSize) {
            return self.capability_denied(request.request_id, "window.size.set");
        }

        match self.window_size.set_size(size) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("applied".to_owned()))]),
            ),
            Err(WindowSizeServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no session window is available for the requested client size.",
                None,
            ),
            Err(WindowSizeServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window size change is already pending.",
                None,
            ),
        }
    }

    /// Asks Windows to foreground this session's own window.
    ///
    /// The payload is exactly `{}` because a target, retry option, native
    /// handle, or input action would turn this narrow request into a general
    /// window or desktop-control surface. The result is only that the host
    /// asked Windows; it deliberately contains no focus or activation state.
    fn handle_window_focus_request(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.focus.request accepts no payload fields.",
                None,
            );
        }
        if !self.policy.has(Capability::WindowFocus) {
            return self.capability_denied(request.request_id, "window.focus.request");
        }

        match self.window_focus.request_focus() {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("requested".to_owned()))]),
            ),
            Err(WindowFocusServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no session window is available for a foreground request.",
                None,
            ),
            Err(WindowFocusServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window focus request is already pending.",
                None,
            ),
        }
    }

    /// Reads every field value on this session's own current surface.
    ///
    /// The payload is exactly `{}`. There is no selector, and that is the
    /// security property rather than a simplification: a caller able to narrow
    /// a read to one field could repeat it until the typing was reconstructed.
    /// Returning the whole surface makes every read cost the same, so reading
    /// often gains nothing. See `docs/UI_FIELDS.md` and Decision 0067.
    fn handle_ui_fields_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "ui.fields.read accepts no payload fields.",
                None,
            );
        }
        if !self.policy.has(Capability::UiFieldsRead) {
            return self.capability_denied(request.request_id, "ui.fields.read");
        }

        match self.ui_fields.read() {
            Ok(snapshot) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([(
                    "fields",
                    JsonValue::Array(
                        snapshot
                            .fields()
                            .iter()
                            .map(|field| {
                                object([
                                    ("id", JsonValue::String(field.id().as_str().to_owned())),
                                    ("value", JsonValue::String(field.value().to_owned())),
                                ])
                            })
                            .collect(),
                    ),
                )]),
            ),
            Err(UiFieldReadError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::UiFieldsUnavailable,
                "no field values are available.",
                None,
            ),
        }
    }

    fn handle_file_dialog_open(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogOpenFile) {
            return self.capability_denied(request.request_id, "dialog.open_file");
        }
        match self.file_dialogs.open_file(&filters) {
            Ok(FileDialogSelection::Selected(path)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("selected".to_owned())),
                    (
                        "path",
                        JsonValue::String(path.as_path().to_string_lossy().into_owned()),
                    ),
                ]),
            ),
            Ok(FileDialogSelection::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            // The typed mailbox prevents this, but an injected host service
            // must never turn a save destination into an open-file result.
            Ok(FileDialogSelection::Saved(_))
            | Ok(FileDialogSelection::Captured(_, _))
            | Ok(FileDialogSelection::CapturedSave(_, _)) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog returned an incompatible result.",
                None,
            ),
            Err(FileDialogServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_dialog_save(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogSaveFile) {
            return self.capability_denied(request.request_id, "dialog.save_file");
        }
        match self.file_dialogs.save_file(&filters) {
            Ok(FileDialogSelection::Saved(path)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("saved".to_owned())),
                    (
                        "path",
                        JsonValue::String(path.as_path().to_string_lossy().into_owned()),
                    ),
                ]),
            ),
            Ok(FileDialogSelection::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Captured(_, _))
            | Ok(FileDialogSelection::CapturedSave(_, _))
            | Err(FileDialogServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_dialog_open_with_reference(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file.v2 requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file.v2 filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogOpenFile) {
            return self.capability_denied(request.request_id, "dialog.open_file");
        }
        match self.file_selections.open_file(&filters) {
            Ok(FileSelectionResult::Selected(selection)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("selected".to_owned())),
                    (
                        "path",
                        JsonValue::String(
                            selection.path().as_path().to_string_lossy().into_owned(),
                        ),
                    ),
                    (
                        "selectionReference",
                        JsonValue::String(selection.reference().as_str().to_owned()),
                    ),
                ]),
            ),
            Ok(FileSelectionResult::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Err(FileSelectionServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_dialog_save_with_reference(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file.v2 requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file.v2 filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogSaveFile) {
            return self.capability_denied(request.request_id, "dialog.save_file");
        }
        match self.file_save_selections.save_file(&filters) {
            Ok(SaveSelectionResult::Selected(selection)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("selected".to_owned())),
                    (
                        "path",
                        JsonValue::String(
                            selection.path().as_path().to_string_lossy().into_owned(),
                        ),
                    ),
                    (
                        "saveReference",
                        JsonValue::String(selection.reference().as_str().to_owned()),
                    ),
                ]),
            ),
            Ok(SaveSelectionResult::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Err(SaveSelectionServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_text_read(&self, request: RequestEnvelope) -> JsonValue {
        let Some(reference) = file_text_read_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "file.read_text requires one exact selection reference.",
                None,
            );
        };
        if !self.policy.has(Capability::FileReadText) {
            return self.capability_denied(request.request_id, "file.read_text");
        }
        match self.file_text.read_text(&reference) {
            Ok(text) if text.len() <= MAX_FILE_TEXT_RESPONSE_BYTES => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("text".to_owned())),
                    ("text", JsonValue::String(text)),
                ]),
            ),
            Ok(_) | Err(FileTextServiceError::TooLarge) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextTooLarge,
                "selected file text is too large.",
                None,
            ),
            Err(FileTextServiceError::InvalidText) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextInvalid,
                "selected file text is invalid.",
                None,
            ),
            Err(FileTextServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileUnavailable,
                "selected file is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_text_write(&self, request: RequestEnvelope) -> JsonValue {
        let Some((reference, text)) = file_text_write_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "file.write_text requires one exact save reference and text.",
                None,
            );
        };
        if text.len() > MAX_FILE_TEXT_WRITE_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextTooLarge,
                "selected output text is too large.",
                None,
            );
        }
        if !self.policy.has(Capability::FileWriteText) {
            return self.capability_denied(request.request_id, "file.write_text");
        }
        match self.file_text_write.write_text(&reference, text) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("written".to_owned()))]),
            ),
            Err(FileTextWriteServiceError::TooLarge) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextTooLarge,
                "selected output text is too large.",
                None,
            ),
            Err(FileTextWriteServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileUnavailable,
                "selected output is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_binary_write(&self, request: RequestEnvelope) -> JsonValue {
        let Some((reference, encoded)) = file_binary_write_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "file.write_binary requires one exact save reference and canonical base64url data.",
                None,
            );
        };
        if !self.policy.has(Capability::FileWriteBinary) {
            return self.capability_denied(request.request_id, "file.write_binary");
        }
        let data = match FileBinaryData::decode_base64url(encoded) {
            Ok(data) => data,
            Err(FileBinaryDataError::Invalid) => {
                self.file_binary_write.discard(&reference);
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "file.write_binary requires canonical base64url data.",
                    None,
                );
            }
            Err(FileBinaryDataError::TooLarge) => {
                self.file_binary_write.discard(&reference);
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::FileBinaryTooLarge,
                    "selected output binary data is too large.",
                    None,
                );
            }
        };
        match self.file_binary_write.write_binary(&reference, &data) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("written".to_owned()))]),
            ),
            Err(FileBinaryWriteServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileUnavailable,
                "selected output is unavailable.",
                None,
            ),
        }
    }

    fn handle_storage_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.read requires an empty payload.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateRead) {
            return self.capability_denied(request.request_id, "storage.state.read");
        }
        match self.storage.read() {
            Ok(StorageRead::Absent) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("absent".to_owned()))]),
            ),
            Ok(StorageRead::Snapshot(snapshot))
                if snapshot.as_str().len() <= MAX_STORAGE_SNAPSHOT_REQUEST_BYTES =>
            {
                ResponseEnvelope::success(
                    request.request_id,
                    &self.policy.host_name,
                    object([
                        ("status", JsonValue::String("snapshot".to_owned())),
                        ("snapshot", JsonValue::String(snapshot.as_str().to_owned())),
                    ]),
                )
            }
            Ok(StorageRead::Snapshot(_)) | Err(StorageServiceError::StoredSnapshotTooLarge) => self
                .storage_failure(
                    request.request_id,
                    StorageServiceError::StoredSnapshotTooLarge,
                ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    fn handle_storage_replace(&self, request: RequestEnvelope) -> JsonValue {
        let Some(snapshot) = storage_replace_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.replace requires one exact snapshot.",
                None,
            );
        };
        if snapshot.len() > MAX_STORAGE_SNAPSHOT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage snapshot is too large.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateReplace) {
            return self.capability_denied(request.request_id, "storage.state.replace");
        }
        let snapshot = match StorageSnapshot::new(snapshot.to_owned()) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "storage snapshot is too large.",
                    None,
                );
            }
        };
        match self.storage.replace(&snapshot) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("replaced".to_owned()))]),
            ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    fn handle_storage_clear(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.clear requires an empty payload.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateClear) {
            return self.capability_denied(request.request_id, "storage.state.clear");
        }
        match self.storage.clear() {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cleared".to_owned()))]),
            ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    fn handle_credential_read(&self, request: RequestEnvelope) -> JsonValue {
        let Some(name) = credential_name_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "credential.read requires one exact credential name.",
                None,
            );
        };
        if !self.policy.has(Capability::CredentialRead) {
            return self.capability_denied(request.request_id, "credential.read");
        }
        match self.credentials.read(&name) {
            Ok(secret) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("found".to_owned())),
                    ("secret", JsonValue::String(secret.to_lower_hex())),
                ]),
            ),
            Err(CredentialServiceError::NotFound) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("not_found".to_owned()))]),
            ),
            Err(error) => self.credential_failure(request.request_id, error),
        }
    }

    fn handle_credential_write(&self, request: RequestEnvelope) -> JsonValue {
        let Some((name, secret)) = credential_write_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "credential.write requires one exact credential name and canonical secret.",
                None,
            );
        };
        if !self.policy.has(Capability::CredentialWrite) {
            return self.capability_denied(request.request_id, "credential.write");
        }
        match self.credentials.write(&name, &secret) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("written".to_owned()))]),
            ),
            Err(error) => self.credential_failure(request.request_id, error),
        }
    }

    fn handle_credential_delete(&self, request: RequestEnvelope) -> JsonValue {
        let Some(name) = credential_name_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "credential.delete requires one exact credential name.",
                None,
            );
        };
        if !self.policy.has(Capability::CredentialDelete) {
            return self.capability_denied(request.request_id, "credential.delete");
        }
        match self.credentials.delete(&name) {
            Ok(true) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("deleted".to_owned()))]),
            ),
            Ok(false) | Err(CredentialServiceError::NotFound) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("not_found".to_owned()))]),
            ),
            Err(error) => self.credential_failure(request.request_id, error),
        }
    }

    fn capability_denied(&self, request_id: String, capability: &str) -> JsonValue {
        self.failure(
            request_id,
            ProtocolErrorCode::CapabilityDenied,
            format!("operation requires the {capability} capability."),
            Some(BTreeMap::from([(
                "capability".to_owned(),
                JsonValue::String(capability.to_owned()),
            )])),
        )
    }

    fn clipboard_failure(&self, request_id: String, error: ClipboardServiceError) -> JsonValue {
        let (code, message) = match error {
            ClipboardServiceError::Unavailable => (
                ProtocolErrorCode::ClipboardUnavailable,
                "clipboard is unavailable.",
            ),
            ClipboardServiceError::StoredTextInvalid => (
                ProtocolErrorCode::ClipboardTextInvalid,
                "clipboard text is invalid.",
            ),
            ClipboardServiceError::StoredTextTooLarge => (
                ProtocolErrorCode::ClipboardTextTooLarge,
                "clipboard text is too large.",
            ),
        };
        self.failure(request_id, code, message, None)
    }

    fn storage_failure(&self, request_id: String, error: StorageServiceError) -> JsonValue {
        let (code, message) = match error {
            StorageServiceError::Unavailable => (
                ProtocolErrorCode::StorageUnavailable,
                "application state is unavailable.",
            ),
            StorageServiceError::StoredSnapshotInvalid => (
                ProtocolErrorCode::StorageSnapshotInvalid,
                "stored application state is invalid.",
            ),
            StorageServiceError::StoredSnapshotTooLarge => (
                ProtocolErrorCode::StorageSnapshotTooLarge,
                "stored application state is too large.",
            ),
        };
        self.failure(request_id, code, message, None)
    }

    fn credential_failure(&self, request_id: String, error: CredentialServiceError) -> JsonValue {
        let (code, message) = match error {
            CredentialServiceError::NotFound => (
                ProtocolErrorCode::CredentialUnavailable,
                "credential service is unavailable.",
            ),
            CredentialServiceError::AccessDenied => (
                ProtocolErrorCode::CredentialAccessDenied,
                "credential access is denied.",
            ),
            CredentialServiceError::Unavailable => (
                ProtocolErrorCode::CredentialUnavailable,
                "credential service is unavailable.",
            ),
            CredentialServiceError::StoredSecretInvalid => (
                ProtocolErrorCode::CredentialStoredSecretInvalid,
                "stored credential is invalid.",
            ),
        };
        self.failure(request_id, code, message, None)
    }

    fn failure(
        &self,
        request_id: String,
        code: ProtocolErrorCode,
        message: impl Into<String>,
        details: Option<BTreeMap<String, JsonValue>>,
    ) -> JsonValue {
        ResponseEnvelope::failure(request_id, &self.policy.host_name, code, message, details)
    }
}

fn ui_action_event(event: anodrel_ui_session::UiApplicationEvent) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("ui.action.invoked".to_owned()),
        ),
        ("source", JsonValue::String("native.ui".to_owned())),
        (
            "schemaVersion",
            ProtocolVersion { major: 1, minor: 0 }.to_json(),
        ),
        (
            "payload",
            object([
                (
                    "revision",
                    JsonValue::String(event.revision().value().to_string()),
                ),
                (
                    "action",
                    JsonValue::String(event.action().as_str().to_owned()),
                ),
            ]),
        ),
    ])
}

/// Builds one v1.25 view-tagged UI action without exposing any native window
/// fact. The tag is an opaque session-local identity, not a handle or a lookup
/// key outside this authenticated group.
fn window_ui_action_event(
    id: &UiWindowId,
    event: anodrel_ui_session::UiApplicationEvent,
) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("ui.action.invoked".to_owned()),
        ),
        ("source", JsonValue::String("native.ui".to_owned())),
        (
            "schemaVersion",
            ProtocolVersion { major: 1, minor: 0 }.to_json(),
        ),
        ("windowId", JsonValue::String(id.to_protocol_string())),
        (
            "payload",
            object([
                (
                    "revision",
                    JsonValue::String(event.revision().value().to_string()),
                ),
                (
                    "action",
                    JsonValue::String(event.action().as_str().to_owned()),
                ),
            ]),
        ),
    ])
}

fn menu_action_event(event: anodrel_menu::MenuActionEvent) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("menu.action.invoked".to_owned()),
        ),
        ("source", JsonValue::String("native.menu".to_owned())),
        ("schemaVersion", MENU_ACTION_EVENT_SCHEMA_VERSION.to_json()),
        (
            "payload",
            object([
                (
                    "menuRevision",
                    JsonValue::String(event.revision().value().to_string()),
                ),
                (
                    "action",
                    JsonValue::String(event.action().as_str().to_owned()),
                ),
            ]),
        ),
    ])
}

/// Builds one v1.25 primary-view-tagged menu action. Menu ownership is still
/// primary-only; the tag makes that fact explicit without reporting a native
/// menu, shortcut, focus state, or window handle.
fn window_menu_action_event(id: &UiWindowId, event: anodrel_menu::MenuActionEvent) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("menu.action.invoked".to_owned()),
        ),
        ("source", JsonValue::String("native.menu".to_owned())),
        ("schemaVersion", MENU_ACTION_EVENT_SCHEMA_VERSION.to_json()),
        ("windowId", JsonValue::String(id.to_protocol_string())),
        (
            "payload",
            object([
                (
                    "menuRevision",
                    JsonValue::String(event.revision().value().to_string()),
                ),
                (
                    "action",
                    JsonValue::String(event.action().as_str().to_owned()),
                ),
            ]),
        ),
    ])
}

fn ui_document_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("document"))
        .flatten()
        .and_then(JsonValue::as_string)
}

/// Reads the exact two-field initial secondary-window payload.
///
/// Extra fields stay invalid so an application cannot smuggle a position,
/// size, parent, native style, handle, or any other desktop control into the
/// first deliberately small window-creation contract.
fn window_open_payload(value: &JsonValue) -> Option<(&str, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    Some((
        fields.get("title")?.as_string()?,
        fields.get("document")?.as_string()?,
    ))
}

/// Reads one exact secondary-window close payload.
///
/// `main` is intentionally rejected here before any host state is consulted:
/// it is the session anchor and ends only through the separately granted
/// `session.close` operation.
fn secondary_window_id_payload(value: &JsonValue) -> Option<UiWindowId> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    let id = UiWindowId::parse(fields.get("windowId")?.as_string()?).ok()?;
    (!id.is_primary()).then_some(id)
}

/// Reads one exact strict-v1 document update targeted at a logical view.
///
/// `main` is allowed, which lets applications use a uniform known-view update
/// method. The host still resolves it only inside the current authenticated
/// group and never exposes a lookup or enumeration operation.
fn window_document_payload(value: &JsonValue) -> Option<(UiWindowId, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    Some((
        UiWindowId::parse(fields.get("windowId")?.as_string()?).ok()?,
        fields.get("document")?.as_string()?,
    ))
}

fn clipboard_write_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("text"))
        .flatten()
        .and_then(JsonValue::as_string)
}

/// Reads the exact two-field payload `notification.show` accepts.
///
/// Any extra field is a mismatch rather than something to ignore, so a future
/// urgency, icon, or action field cannot be smuggled past this version.
fn notification_show_payload(value: &JsonValue) -> Option<(&str, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let title = fields.get("title").and_then(JsonValue::as_string)?;
    let body = fields.get("body").and_then(JsonValue::as_string)?;
    Some((title, body))
}

/// Reads the exact one-field payload `window.title.set` accepts.
///
/// Any extra field is a mismatch rather than something to ignore, so a future
/// window target, identifier, position, or size cannot be smuggled past this
/// version — which is the whole reason the capability is safe at all.
fn window_title_set_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("title"))
        .flatten()
        .and_then(JsonValue::as_string)
}

/// Reads the exact one-field payload `window.state.set` accepts.
///
/// Extra fields are a mismatch rather than a future target, geometry, focus, or
/// native-command escape hatch. The value itself is a closed portable enum, so
/// the core never receives an operating-system state code.
fn window_state_set_payload(value: &JsonValue) -> Option<WindowState> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    match fields.get("state")?.as_string()? {
        "minimized" => Some(WindowState::Minimized),
        "maximized" => Some(WindowState::Maximized),
        "restored" => Some(WindowState::Restored),
        _ => None,
    }
}

/// Reads the exact one-field payload `window.fullscreen.set` accepts.
///
/// Extra fields are a mismatch rather than a future monitor, display-mode,
/// geometry, style, z-order, or input escape hatch. The value itself remains a
/// closed portable mode, so the core never receives a native presentation code.
fn window_fullscreen_set_payload(value: &JsonValue) -> Option<WindowFullscreenMode> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    match fields.get("mode")?.as_string()? {
        "fullscreen" => Some(WindowFullscreenMode::Fullscreen),
        "windowed" => Some(WindowFullscreenMode::Windowed),
        _ => None,
    }
}

/// Reads the exact two-field payload `window.size.set` accepts.
///
/// A position, target, monitor, native rectangle, DPI, constraint, animation,
/// or readback selector must not be smuggled into the small client-size command.
/// Both values are strict non-negative JSON integers before the portable bounded
/// logical client-area type accepts them.
fn window_size_set_payload(value: &JsonValue) -> Option<WindowSize> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let width = u32::from(fields.get("width")?.as_u16()?);
    let height = u32::from(fields.get("height")?.as_u16()?);
    WindowSize::new(width, height).ok()
}

fn external_open_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("url"))
        .flatten()
        .and_then(JsonValue::as_string)
}

/// Reads the exact one-field payload `network.fetch_text` accepts.
///
/// Extra fields are a mismatch rather than a future method, body, header,
/// cookie, credential, proxy, redirect, timeout, or native-handle escape
/// hatch. That absence keeps the service a bounded data seam.
fn network_fetch_text_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("url"))
        .flatten()
        .and_then(JsonValue::as_string)
}

fn file_dialog_open_payload(value: &JsonValue) -> Option<Vec<FileDialogFilter>> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    let JsonValue::Array(filters) = fields.get("filters")? else {
        return None;
    };
    if filters.is_empty() || filters.len() > MAX_FILE_DIALOG_FILTERS {
        return None;
    }
    filters
        .iter()
        .map(|filter| {
            let fields = filter.as_object()?;
            if fields.len() != 2 {
                return None;
            }
            let label = fields.get("label")?.as_string()?.to_owned();
            let JsonValue::Array(extensions) = fields.get("extensions")? else {
                return None;
            };
            let extensions = extensions
                .iter()
                .map(|extension| extension.as_string().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?;
            FileDialogFilter::new(label, extensions).ok()
        })
        .collect()
}

fn file_text_read_payload(value: &JsonValue) -> Option<SelectionReference> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    SelectionReference::new(fields.get("selectionReference")?.as_string()?.to_owned()).ok()
}

fn file_text_write_payload(value: &JsonValue) -> Option<(SaveReference, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let reference =
        SaveReference::new(fields.get("saveReference")?.as_string()?.to_owned()).ok()?;
    let text = fields.get("text")?.as_string()?;
    Some((reference, text))
}

fn file_binary_write_payload(value: &JsonValue) -> Option<(SaveReference, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let reference =
        SaveReference::new(fields.get("saveReference")?.as_string()?.to_owned()).ok()?;
    let encoded = fields.get("bytesBase64Url")?.as_string()?;
    Some((reference, encoded))
}

fn menu_replace_payload(value: &JsonValue, shortcuts_allowed: bool) -> Option<MenuModel> {
    if value.to_json().len() > MAX_MENU_REPLACE_REQUEST_BYTES {
        return None;
    }
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    let JsonValue::Array(menus) = fields.get("menus")? else {
        return None;
    };
    let menus = menus
        .iter()
        .map(|menu| {
            let fields = menu.as_object()?;
            if fields.len() != 2 {
                return None;
            }
            let label = MenuText::new(fields.get("label")?.as_string()?.to_owned()).ok()?;
            let JsonValue::Array(items) = fields.get("items")? else {
                return None;
            };
            let items = items
                .iter()
                .map(|item| {
                    let fields = item.as_object()?;
                    let shortcut = fields.get("shortcut");
                    if fields.len() != 3 + usize::from(shortcut.is_some())
                        || (!shortcuts_allowed && shortcut.is_some())
                    {
                        return None;
                    }
                    let id = MenuActionId::new(fields.get("id")?.as_string()?.to_owned()).ok()?;
                    let label = MenuText::new(fields.get("label")?.as_string()?.to_owned()).ok()?;
                    let JsonValue::Bool(enabled) = fields.get("enabled")? else {
                        return None;
                    };
                    let action = MenuAction::new(id, label, *enabled);
                    match shortcut {
                        Some(shortcut) => Some(
                            action.with_shortcut(MenuShortcut::parse(shortcut.as_string()?).ok()?),
                        ),
                        None => Some(action),
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            Menu::new(label, items).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    MenuModel::new(menus).ok()
}

fn storage_replace_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("snapshot"))
        .flatten()
        .and_then(JsonValue::as_string)
}

fn credential_name_payload(value: &JsonValue) -> Option<CredentialName> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("name"))
        .flatten()
        .and_then(JsonValue::as_string)
        .and_then(|name| CredentialName::parse(name).ok())
}

fn credential_write_payload(value: &JsonValue) -> Option<(CredentialName, Secret)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let name = CredentialName::parse(fields.get("name")?.as_string()?).ok()?;
    let secret = Secret::from_lower_hex(fields.get("secret")?.as_string()?).ok()?;
    Some((name, secret))
}

fn rfc3339_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = duration.as_secs().min(i64::MAX as u64) as i64;
    let milliseconds = duration.subsec_millis();
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milliseconds:03}Z")
}

// Howard Hinnant's public-domain civil-date conversion, expressed here with
// integer arithmetic so the runtime does not need a time-formatting library.
fn civil_from_days(days_since_unix_epoch: i64) -> (i64, u32, u32) {
    let shifted = days_since_unix_epoch + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use anodrel_clipboard::{
        ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText,
    };
    use anodrel_credentials::{CredentialName, CredentialService, CredentialServiceError, Secret};
    use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
    use anodrel_file_access::{
        FileBinaryData, FileBinaryWriteService, FileBinaryWriteServiceError, FileSelection,
        FileSelectionService, FileTextService, FileTextWriteService, FileTextWriteServiceError,
        SaveReference, SaveSelection, SaveSelectionResult, SaveSelectionService,
        SaveSelectionServiceError,
    };
    use anodrel_file_dialog::{SaveFilePath, SelectedFilePath};
    use anodrel_menu::{MenuModel, MenuRevision, MenuService, MenuServiceError};
    use anodrel_network::{
        NetworkTextResponse, NetworkTextService, NetworkTextServiceError, NetworkUrl,
    };
    use anodrel_notifications::{Notification, NotificationService, NotificationServiceError};
    use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};
    use anodrel_ui::{ElementId, UiEvent};
    use anodrel_ui_session::{
        MenuInputCandidate, UiDocumentMailbox, UiInputCandidate, UiInputMailbox, UiWindowGroup,
    };
    use anodrel_window::{
        WindowFocusService, WindowFocusServiceError, WindowFullscreenMode, WindowFullscreenService,
        WindowFullscreenServiceError, WindowSize, WindowSizeService, WindowSizeServiceError,
        WindowState, WindowStateService, WindowStateServiceError, WindowTitleProposal,
        WindowTitleService, WindowTitleServiceError,
    };

    use super::*;

    fn host(grants: Vec<Capability>) -> CoreHost {
        CoreHost::new(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        )
    }

    #[derive(Debug)]
    struct MemoryClipboard {
        text: RefCell<Option<ClipboardText>>,
    }

    impl MemoryClipboard {
        fn with_text(text: Option<&str>) -> Self {
            Self {
                text: RefCell::new(
                    text.map(|value| ClipboardText::new(value).expect("fixture text")),
                ),
            }
        }
    }

    impl ClipboardService for MemoryClipboard {
        fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
            Ok(self
                .text
                .borrow()
                .clone()
                .map(ClipboardRead::Text)
                .unwrap_or(ClipboardRead::NoText))
        }

        fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardServiceError> {
            *self.text.borrow_mut() = Some(text.clone());
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingClipboard(ClipboardServiceError);

    impl ClipboardService for FailingClipboard {
        fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
            Err(self.0)
        }

        fn write_text(&self, _text: &ClipboardText) -> Result<(), ClipboardServiceError> {
            Err(self.0)
        }
    }

    #[derive(Debug, Default)]
    struct RecordingExternalLinks(RefCell<Option<ExternalLink>>);

    impl ExternalLinkService for RecordingExternalLinks {
        fn open(&self, link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
            *self.0.borrow_mut() = Some(link.clone());
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingExternalLinks;

    impl ExternalLinkService for FailingExternalLinks {
        fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
            Err(ExternalLinkOpenError::Unavailable)
        }
    }

    #[derive(Clone, Debug)]
    struct RecordingNetwork {
        requested: Arc<Mutex<Vec<NetworkUrl>>>,
        result: Result<NetworkTextResponse, NetworkTextServiceError>,
    }

    impl RecordingNetwork {
        fn responding(status_code: u16, text: &str) -> Self {
            Self {
                requested: Arc::new(Mutex::new(Vec::new())),
                result: Ok(NetworkTextResponse::new(status_code, text)
                    .expect("network fixture response is valid")),
            }
        }

        fn failing(error: NetworkTextServiceError) -> Self {
            Self {
                requested: Arc::new(Mutex::new(Vec::new())),
                result: Err(error),
            }
        }
    }

    impl NetworkTextService for RecordingNetwork {
        fn fetch_text(
            &self,
            url: &NetworkUrl,
        ) -> Result<NetworkTextResponse, NetworkTextServiceError> {
            self.requested
                .lock()
                .expect("network recorder lock is available")
                .push(url.clone());
            self.result.clone()
        }
    }

    #[derive(Debug)]
    struct CancellingFileDialog;

    impl FileDialogService for CancellingFileDialog {
        fn open_file(
            &self,
            _filters: &[FileDialogFilter],
        ) -> Result<FileDialogSelection, FileDialogServiceError> {
            Ok(FileDialogSelection::Cancelled)
        }
    }

    #[derive(Debug)]
    struct SavingFileDialog;

    impl FileDialogService for SavingFileDialog {
        fn open_file(
            &self,
            _filters: &[FileDialogFilter],
        ) -> Result<FileDialogSelection, FileDialogServiceError> {
            Ok(FileDialogSelection::Cancelled)
        }

        fn save_file(
            &self,
            _filters: &[FileDialogFilter],
        ) -> Result<FileDialogSelection, FileDialogServiceError> {
            let destination = std::env::temp_dir().join("anodrel-save-dialog-test.txt");
            Ok(FileDialogSelection::Saved(
                SaveFilePath::new(destination).expect("temporary directory is absolute"),
            ))
        }
    }

    #[derive(Debug)]
    struct CapturingFileDialog;

    impl FileSelectionService for CapturingFileDialog {
        fn open_file(
            &self,
            _filters: &[FileDialogFilter],
        ) -> Result<FileSelectionResult, FileSelectionServiceError> {
            let path =
                SelectedFilePath::new(r"C:\\Users\\Owner\\selection.txt").expect("path is valid");
            let reference =
                SelectionReference::new("AbCdEfGhIjKlMnOpQrStUv").expect("reference is valid");
            Ok(FileSelectionResult::Selected(FileSelection::new(
                path, reference,
            )))
        }
    }

    #[derive(Debug)]
    struct FixedFileText(Result<String, FileTextServiceError>);

    impl FileTextService for FixedFileText {
        fn read_text(
            &self,
            _reference: &SelectionReference,
        ) -> Result<String, FileTextServiceError> {
            self.0.clone()
        }
    }

    #[derive(Debug)]
    struct CapturingSaveDialog;

    impl SaveSelectionService for CapturingSaveDialog {
        fn save_file(
            &self,
            _filters: &[FileDialogFilter],
        ) -> Result<SaveSelectionResult, SaveSelectionServiceError> {
            let path = SaveFilePath::new(r"C:\\Users\\Owner\\save.txt").expect("path is valid");
            let reference =
                SaveReference::new("ZyXwVuTsRqPoNmLkJiHgFe").expect("reference is valid");
            Ok(SaveSelectionResult::Selected(SaveSelection::new(
                path, reference,
            )))
        }
    }

    #[derive(Clone, Debug)]
    struct RecordingFileTextWrite {
        writes: Arc<Mutex<Vec<String>>>,
        result: Result<(), FileTextWriteServiceError>,
    }

    impl RecordingFileTextWrite {
        fn accepting() -> Self {
            Self {
                writes: Arc::new(Mutex::new(Vec::new())),
                result: Ok(()),
            }
        }

        fn failing(error: FileTextWriteServiceError) -> Self {
            Self {
                writes: Arc::new(Mutex::new(Vec::new())),
                result: Err(error),
            }
        }
    }

    impl FileTextWriteService for RecordingFileTextWrite {
        fn write_text(
            &self,
            _reference: &SaveReference,
            text: &str,
        ) -> Result<(), FileTextWriteServiceError> {
            self.writes
                .lock()
                .expect("write recorder lock is available")
                .push(text.to_owned());
            self.result
        }
    }

    #[derive(Clone, Debug)]
    struct RecordingFileBinaryWrite {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
        discarded: Arc<Mutex<Vec<SaveReference>>>,
        result: Result<(), FileBinaryWriteServiceError>,
    }

    impl RecordingFileBinaryWrite {
        fn accepting() -> Self {
            Self {
                writes: Arc::new(Mutex::new(Vec::new())),
                discarded: Arc::new(Mutex::new(Vec::new())),
                result: Ok(()),
            }
        }

        fn unavailable() -> Self {
            Self {
                writes: Arc::new(Mutex::new(Vec::new())),
                discarded: Arc::new(Mutex::new(Vec::new())),
                result: Err(FileBinaryWriteServiceError::Unavailable),
            }
        }
    }

    impl FileBinaryWriteService for RecordingFileBinaryWrite {
        fn write_binary(
            &self,
            _reference: &SaveReference,
            data: &FileBinaryData,
        ) -> Result<(), FileBinaryWriteServiceError> {
            self.writes
                .lock()
                .expect("binary-write recorder lock is available")
                .push(data.as_bytes().to_vec());
            self.result
        }

        fn discard(&self, reference: &SaveReference) {
            self.discarded
                .lock()
                .expect("binary-discard recorder lock is available")
                .push(reference.clone());
        }
    }

    #[derive(Clone, Debug)]
    struct RecordingMenu {
        replacements: Arc<Mutex<Vec<(MenuRevision, MenuModel)>>>,
        result: Result<(), MenuServiceError>,
    }

    impl Default for RecordingMenu {
        fn default() -> Self {
            Self {
                replacements: Arc::new(Mutex::new(Vec::new())),
                result: Ok(()),
            }
        }
    }

    impl RecordingMenu {
        fn unavailable() -> Self {
            Self {
                replacements: Arc::new(Mutex::new(Vec::new())),
                result: Err(MenuServiceError::Unavailable),
            }
        }
    }

    impl MenuService for RecordingMenu {
        fn replace(
            &self,
            revision: MenuRevision,
            model: MenuModel,
        ) -> Result<(), MenuServiceError> {
            if self.result.is_ok() {
                self.replacements
                    .lock()
                    .expect("menu recorder lock is available")
                    .push((revision, model));
            }
            self.result
        }
    }

    #[derive(Debug)]
    struct MemoryStorage(Mutex<Result<StorageRead, StorageServiceError>>);

    impl MemoryStorage {
        fn with_state(state: StorageRead) -> Self {
            Self(Mutex::new(Ok(state)))
        }
    }

    impl StorageService for MemoryStorage {
        fn read(&self) -> Result<StorageRead, StorageServiceError> {
            self.0.lock().expect("storage lock is available").clone()
        }

        fn replace(&self, snapshot: &StorageSnapshot) -> Result<(), StorageServiceError> {
            *self.0.lock().expect("storage lock is available") =
                Ok(StorageRead::Snapshot(snapshot.clone()));
            Ok(())
        }

        fn clear(&self) -> Result<(), StorageServiceError> {
            *self.0.lock().expect("storage lock is available") = Ok(StorageRead::Absent);
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct MemoryCredentials(Mutex<BTreeMap<String, Vec<u8>>>);

    impl CredentialService for MemoryCredentials {
        fn read(&self, name: &CredentialName) -> Result<Secret, CredentialServiceError> {
            self.0
                .lock()
                .expect("credential lock is available")
                .get(name.as_str())
                .cloned()
                .map(|bytes| Secret::new(bytes).expect("stored fixture secret is valid"))
                .ok_or(CredentialServiceError::NotFound)
        }

        fn write(
            &self,
            name: &CredentialName,
            secret: &Secret,
        ) -> Result<(), CredentialServiceError> {
            self.0
                .lock()
                .expect("credential lock is available")
                .insert(name.as_str().to_owned(), secret.as_bytes().to_vec());
            Ok(())
        }

        fn delete(&self, name: &CredentialName) -> Result<bool, CredentialServiceError> {
            Ok(self
                .0
                .lock()
                .expect("credential lock is available")
                .remove(name.as_str())
                .is_some())
        }
    }

    fn clipboard_host(
        grants: Vec<Capability>,
        clipboard: impl ClipboardService + 'static,
    ) -> CoreHost {
        CoreHost::with_session_components_and_clipboard(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            clipboard,
        )
    }

    fn external_host(
        grants: Vec<Capability>,
        external_links: impl ExternalLinkService + 'static,
    ) -> CoreHost {
        CoreHost::with_session_components_and_services(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            MemoryClipboard::with_text(None),
            external_links,
        )
    }

    fn network_host(
        grants: Vec<Capability>,
        network: impl NetworkTextService + 'static,
    ) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            HostServices::unavailable().with_network(network),
        )
    }

    fn file_dialog_host(
        grants: Vec<Capability>,
        dialogs: impl FileDialogService + 'static,
    ) -> CoreHost {
        CoreHost::with_session_components_and_all_services(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            MemoryClipboard::with_text(None),
            FailingExternalLinks,
            dialogs,
        )
    }

    fn file_access_host(
        grants: Vec<Capability>,
        selections: impl FileSelectionService + 'static,
        text: impl FileTextService + 'static,
    ) -> CoreHost {
        CoreHost::with_session_components_and_all_services_and_file_access(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            MemoryClipboard::with_text(None),
            FailingExternalLinks,
            CancellingFileDialog,
            selections,
            text,
        )
    }

    fn file_write_host(
        grants: Vec<Capability>,
        selections: impl SaveSelectionService + 'static,
        writer: impl FileTextWriteService + 'static,
    ) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            HostServices::unavailable()
                .with_file_save_selections(selections)
                .with_file_text_write(writer),
        )
    }

    fn file_binary_write_host(
        grants: Vec<Capability>,
        writer: impl FileBinaryWriteService + 'static,
    ) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            HostServices::unavailable().with_file_binary_write(writer),
        )
    }

    fn storage_host(grants: Vec<Capability>, storage: impl StorageService + 'static) -> CoreHost {
        CoreHost::with_session_components_and_all_services_and_file_access_and_storage(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            MemoryClipboard::with_text(None),
            FailingExternalLinks,
            CancellingFileDialog,
            CapturingFileDialog,
            FixedFileText(Err(FileTextServiceError::Unavailable)),
            storage,
        )
    }

    fn credential_host(
        grants: Vec<Capability>,
        credentials: impl CredentialService + 'static,
    ) -> CoreHost {
        CoreHost::with_credential_service(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
            credentials,
        )
    }

    fn request(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":0}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_1(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":1}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_2(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":2}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_3(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":3}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_4(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":4}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_5(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":5}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_6(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":6}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_7(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":7}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_8(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":8}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_9(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":9}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_10(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":10}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_12(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":12}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn host_with_notifications(service: impl NotificationService + 'static) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::NotificationShow],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_notifications(service),
        )
    }

    fn request_v1_13(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":13}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    /// A notification service that records what it was asked to show.
    #[derive(Debug, Default)]
    struct RecordingNotifications {
        shown: std::sync::Mutex<Vec<(String, String)>>,
        result: Option<NotificationServiceError>,
    }

    impl RecordingNotifications {
        fn failing(error: NotificationServiceError) -> Self {
            Self {
                shown: std::sync::Mutex::new(Vec::new()),
                result: Some(error),
            }
        }
    }

    impl NotificationService for RecordingNotifications {
        fn show(&self, notification: &Notification) -> Result<(), NotificationServiceError> {
            if let Some(error) = self.result {
                return Err(error);
            }
            self.shown
                .lock()
                .expect("the fixture lock is usable")
                .push((
                    notification.title().as_str().to_owned(),
                    notification.body().as_str().to_owned(),
                ));
            Ok(())
        }
    }

    fn notification_payload(title: &str, body: &str) -> String {
        object([
            ("body", JsonValue::String(body.to_owned())),
            ("title", JsonValue::String(title.to_owned())),
        ])
        .to_json()
    }

    fn ui_document_payload(document: &str) -> String {
        object([("document", JsonValue::String(document.to_owned()))]).to_json()
    }

    fn valid_ui_document(label: &str) -> String {
        format!(
            r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"root","kind":"action","label":"{label}","fontSize":16,"enabled":true,"tone":"accent"}}}}"#
        )
    }

    fn valid_ui_document_v2() -> &'static str {
        r#"{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"content","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}}"#
    }

    fn valid_ui_document_v3(value: &str, politeness: &str) -> String {
        format!(
            r#"{{"format":"anodrel.ui.document.v3","root":{{"id":"status","kind":"status","value":"{value}","fontSize":16,"tone":"accent","politeness":"{politeness}"}}}}"#
        )
    }

    fn field<'a>(value: &'a JsonValue, field: &str) -> &'a JsonValue {
        &value.as_object().expect("response is an object")[field]
    }

    #[test]
    fn notifications_need_their_own_grant_and_protocol_minor() {
        let payload = notification_payload("Build finished", "Two targets");

        // No grant.
        let denied = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new("test.application", vec![], "test-host").expect("policy is valid"),
                HostServices::unavailable().with_notifications(RecordingNotifications::default()),
            )
            .handle_json(&request_v1_13("notification.show", &payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        // Granted, but asked for at a protocol minor that predates the
        // operation: an older client must not reach a newer capability.
        let unsupported = JsonValue::parse(
            &host_with_notifications(RecordingNotifications::default())
                .handle_json(&request_v1_12("notification.show", &payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    /// A window-title service that records the proposals it was handed.
    #[derive(Debug, Default)]
    struct RecordingWindowTitle {
        applied: std::sync::Mutex<Vec<String>>,
        result: Option<WindowTitleServiceError>,
    }

    impl WindowTitleService for RecordingWindowTitle {
        fn set_title(&self, proposal: &WindowTitleProposal) -> Result<(), WindowTitleServiceError> {
            if let Some(error) = self.result {
                return Err(error);
            }
            self.applied
                .lock()
                .expect("the test mutex is usable")
                .push(proposal.as_str().to_owned());
            Ok(())
        }
    }

    fn request_v1_14(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":14}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn host_with_window_title(service: impl WindowTitleService + 'static) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowTitle],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_title(service),
        )
    }

    #[test]
    fn a_granted_window_title_reaches_the_service_unchanged() {
        let service = RecordingWindowTitle::default();
        let response = JsonValue::parse(&host_with_window_title(service).handle_json(
            &request_v1_14("window.title.set", r#"{"title":"Quarterly Report.pdf"}"#),
        ))
        .expect("response JSON is valid");

        assert_eq!(field(&response, "status").as_string(), Some("success"));
        // Acceptance only. The composed caption is deliberately not returned:
        // it would hand the application the host's framing format to probe.
        assert_eq!(
            field(field(&response, "result"), "status").as_string(),
            Some("applied")
        );
    }

    #[test]
    fn a_window_title_needs_its_own_grant_and_its_own_protocol_version() {
        let payload = r#"{"title":"Report"}"#;

        // Held every other grant, but not this one.
        let denied = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::NotificationShow, Capability::DiagnosticsRead],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable().with_window_title(RecordingWindowTitle::default()),
            )
            .handle_json(&request_v1_14("window.title.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        // Granted, but asked for at a protocol minor that predates it.
        let unsupported = JsonValue::parse(
            &host_with_window_title(RecordingWindowTitle::default())
                .handle_json(&request_v1_13("window.title.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn a_window_title_payload_accepts_exactly_one_title_field() {
        // An extra field is a mismatch rather than something to ignore, so a
        // future target, identifier, position, or size cannot be smuggled past
        // this version. That refusal is what keeps the capability un-aimable.
        for payload in [
            r#"{}"#,
            r#"{"title":"Report","target":"other-window"}"#,
            r#"{"title":"Report","windowId":2}"#,
            r#"{"caption":"Report"}"#,
            r#"{"title":7}"#,
        ] {
            let response = JsonValue::parse(
                &host_with_window_title(RecordingWindowTitle::default())
                    .handle_json(&request_v1_14("window.title.set", payload)),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
        }
    }

    #[test]
    fn a_rejected_window_title_never_echoes_the_text_it_refused() {
        // The marker is what a leak would look like: text refused for being
        // unsafe to display must not be repeated in an error that reaches logs.
        let marker = "MarkerZQX";
        let response = host_with_window_title(RecordingWindowTitle::default()).handle_json(
            &request_v1_14("window.title.set", &format!(r#"{{"title":"{marker}\n"}}"#)),
        );
        let parsed = JsonValue::parse(&response).expect("response JSON is valid");
        assert_eq!(
            field(field(&parsed, "error"), "code").as_string(),
            Some("window.title_invalid")
        );
        assert!(!response.contains(marker), "the refused title was echoed");
    }

    #[test]
    fn window_title_service_failures_map_to_their_own_codes() {
        for (error, code) in [
            (WindowTitleServiceError::Unavailable, "window.unavailable"),
            (WindowTitleServiceError::Busy, "window.busy"),
        ] {
            let service = RecordingWindowTitle {
                result: Some(error),
                ..RecordingWindowTitle::default()
            };
            let response = JsonValue::parse(
                &host_with_window_title(service)
                    .handle_json(&request_v1_14("window.title.set", r#"{"title":"Report"}"#)),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(code),
                "{error:?} mapped to the wrong code"
            );
        }
    }

    #[test]
    fn a_host_without_a_window_title_service_reports_unavailable() {
        let response = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowTitle],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable(),
            )
            .handle_json(&request_v1_14("window.title.set", r#"{"title":"Report"}"#)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("window.unavailable")
        );
    }

    /// A state service that records only the closed command it was given.
    #[derive(Debug, Default)]
    struct RecordingWindowState {
        applied: Arc<Mutex<Vec<WindowState>>>,
        result: Option<WindowStateServiceError>,
    }

    impl WindowStateService for RecordingWindowState {
        fn set_state(&self, state: WindowState) -> Result<(), WindowStateServiceError> {
            if let Some(error) = self.result {
                return Err(error);
            }
            self.applied
                .lock()
                .expect("the test mutex is usable")
                .push(state);
            Ok(())
        }
    }

    fn request_v1_16(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":16}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_17(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":17}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_18(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":18}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_24(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":24}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_25(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":25}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_26(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":26}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_19(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":19}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_20(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":20}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_21(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":21}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_22(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":22}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_23(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":23}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn host_with_menu(service: impl MenuService + 'static) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new("test.application", vec![Capability::MenuWrite], "test-host")
                .expect("test policy is valid"),
            HostServices::unavailable().with_menu(service),
        )
    }

    fn host_with_window_state(service: impl WindowStateService + 'static) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowState],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_state(service),
        )
    }

    #[derive(Debug, Default)]
    struct RecordingWindowFocus {
        requested: Arc<Mutex<u8>>,
        result: Option<WindowFocusServiceError>,
    }

    #[derive(Debug, Default)]
    struct RecordingWindowFullscreen {
        applied: Arc<Mutex<Vec<WindowFullscreenMode>>>,
        result: Option<WindowFullscreenServiceError>,
    }

    impl WindowFullscreenService for RecordingWindowFullscreen {
        fn set_fullscreen(
            &self,
            mode: WindowFullscreenMode,
        ) -> Result<(), WindowFullscreenServiceError> {
            if let Some(error) = self.result {
                return Err(error);
            }
            self.applied
                .lock()
                .expect("the test mutex is usable")
                .push(mode);
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingWindowSize {
        applied: Arc<Mutex<Vec<WindowSize>>>,
        result: Option<WindowSizeServiceError>,
    }

    impl WindowSizeService for RecordingWindowSize {
        fn set_size(&self, size: WindowSize) -> Result<(), WindowSizeServiceError> {
            if let Some(error) = self.result {
                return Err(error);
            }
            self.applied
                .lock()
                .expect("the test mutex is usable")
                .push(size);
            Ok(())
        }
    }

    impl WindowFocusService for RecordingWindowFocus {
        fn request_focus(&self) -> Result<(), WindowFocusServiceError> {
            if let Some(error) = self.result {
                return Err(error);
            }
            let requested = &mut *self.requested.lock().expect("the test mutex is usable");
            *requested = requested.saturating_add(1);
            Ok(())
        }
    }

    fn host_with_window_focus(service: impl WindowFocusService + 'static) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowFocus],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_focus(service),
        )
    }

    fn host_with_window_fullscreen(service: impl WindowFullscreenService + 'static) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowFullscreen],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_fullscreen(service),
        )
    }

    fn host_with_window_size(service: impl WindowSizeService + 'static) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowSize],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_window_size(service),
        )
    }

    #[test]
    fn every_granted_window_state_reaches_only_the_state_service() {
        for (payload, expected) in [
            (r#"{"state":"minimized"}"#, WindowState::Minimized),
            (r#"{"state":"maximized"}"#, WindowState::Maximized),
            (r#"{"state":"restored"}"#, WindowState::Restored),
        ] {
            let service = RecordingWindowState::default();
            let applied = Arc::clone(&service.applied);
            let response = JsonValue::parse(
                &host_with_window_state(service)
                    .handle_json(&request_v1_16("window.state.set", payload)),
            )
            .expect("response JSON is valid");
            assert_eq!(field(&response, "status").as_string(), Some("success"));
            assert_eq!(
                field(field(&response, "result"), "status").as_string(),
                Some("applied"),
                "{expected:?} did not report acceptance"
            );
            assert_eq!(
                applied.lock().expect("the test mutex is usable").as_slice(),
                &[expected]
            );
        }
    }

    #[test]
    fn window_state_needs_its_own_grant_and_protocol_version() {
        let payload = r#"{"state":"minimized"}"#;
        let denied = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowTitle, Capability::UiFieldsRead],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable().with_window_state(RecordingWindowState::default()),
            )
            .handle_json(&request_v1_16("window.state.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let unsupported = JsonValue::parse(
            &host_with_window_state(RecordingWindowState::default())
                .handle_json(&request_v1_15("window.state.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn window_state_payload_is_exact_and_closed() {
        for payload in [
            r#"{}"#,
            r#"{"state":"fullscreen"}"#,
            r#"{"state":"minimized","target":"another-window"}"#,
            r#"{"state":"restored","bounds":{"width":1}}"#,
            r#"{"state":7}"#,
        ] {
            let response = JsonValue::parse(
                &host_with_window_state(RecordingWindowState::default())
                    .handle_json(&request_v1_16("window.state.set", payload)),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
        }
    }

    #[test]
    fn window_state_service_failures_map_to_safe_shared_codes() {
        for (error, code) in [
            (WindowStateServiceError::Unavailable, "window.unavailable"),
            (WindowStateServiceError::Busy, "window.busy"),
        ] {
            let response = JsonValue::parse(
                &host_with_window_state(RecordingWindowState {
                    result: Some(error),
                    ..RecordingWindowState::default()
                })
                .handle_json(&request_v1_16(
                    "window.state.set",
                    r#"{"state":"restored"}"#,
                )),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(code),
                "{error:?} mapped to the wrong code"
            );
        }
    }

    #[test]
    fn a_host_without_a_window_state_service_reports_unavailable() {
        let response = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowState],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable(),
            )
            .handle_json(&request_v1_16(
                "window.state.set",
                r#"{"state":"maximized"}"#,
            )),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("window.unavailable")
        );
    }

    #[test]
    fn a_granted_window_focus_request_reaches_only_the_focus_service() {
        let service = RecordingWindowFocus::default();
        let requested = Arc::clone(&service.requested);
        let response = JsonValue::parse(
            &host_with_window_focus(service)
                .handle_json(&request_v1_20("window.focus.request", r#"{}"#)),
        )
        .expect("response JSON is valid");

        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&response, "result"), "status").as_string(),
            Some("requested")
        );
        assert_eq!(*requested.lock().expect("the test mutex is usable"), 1);
    }

    #[test]
    fn window_focus_needs_its_own_grant_and_protocol_version() {
        let denied = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowState, Capability::WindowTitle],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable().with_window_focus(RecordingWindowFocus::default()),
            )
            .handle_json(&request_v1_20("window.focus.request", r#"{}"#)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let unsupported = JsonValue::parse(
            &host_with_window_focus(RecordingWindowFocus::default())
                .handle_json(&request_v1_19("window.focus.request", r#"{}"#)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn window_focus_payload_is_exact_and_untargetable() {
        for payload in [
            r#"null"#,
            r#"{"target":"another-window"}"#,
            r#"{"handle":7}"#,
            r#"{"retry":true}"#,
            r#"{"input":"click"}"#,
        ] {
            let response = JsonValue::parse(
                &host_with_window_focus(RecordingWindowFocus::default())
                    .handle_json(&request_v1_20("window.focus.request", payload)),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
        }
    }

    #[test]
    fn window_focus_service_failures_map_to_safe_shared_codes() {
        for (error, code) in [
            (WindowFocusServiceError::Unavailable, "window.unavailable"),
            (WindowFocusServiceError::Busy, "window.busy"),
        ] {
            let response = JsonValue::parse(
                &host_with_window_focus(RecordingWindowFocus {
                    result: Some(error),
                    ..RecordingWindowFocus::default()
                })
                .handle_json(&request_v1_20("window.focus.request", r#"{}"#)),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(code),
                "{error:?} mapped to the wrong code"
            );
        }
    }

    #[test]
    fn a_host_without_a_window_focus_service_reports_unavailable() {
        let response = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowFocus],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable(),
            )
            .handle_json(&request_v1_20("window.focus.request", r#"{}"#)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("window.unavailable")
        );
    }

    #[test]
    fn every_granted_window_fullscreen_mode_reaches_only_the_fullscreen_service() {
        for (payload, expected) in [
            (r#"{"mode":"fullscreen"}"#, WindowFullscreenMode::Fullscreen),
            (r#"{"mode":"windowed"}"#, WindowFullscreenMode::Windowed),
        ] {
            let service = RecordingWindowFullscreen::default();
            let applied = Arc::clone(&service.applied);
            let response = JsonValue::parse(
                &host_with_window_fullscreen(service)
                    .handle_json(&request_v1_21("window.fullscreen.set", payload)),
            )
            .expect("response JSON is valid");
            assert_eq!(field(&response, "status").as_string(), Some("success"));
            assert_eq!(
                field(field(&response, "result"), "status").as_string(),
                Some("applied")
            );
            assert_eq!(
                applied.lock().expect("the test mutex is usable").as_slice(),
                &[expected]
            );
        }
    }

    #[test]
    fn window_fullscreen_needs_its_own_grant_and_protocol_version() {
        let payload = r#"{"mode":"fullscreen"}"#;
        let denied = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowFocus, Capability::WindowState],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable()
                    .with_window_fullscreen(RecordingWindowFullscreen::default()),
            )
            .handle_json(&request_v1_21("window.fullscreen.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let unsupported = JsonValue::parse(
            &host_with_window_fullscreen(RecordingWindowFullscreen::default())
                .handle_json(&request_v1_20("window.fullscreen.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn window_fullscreen_payload_is_exact_and_closed() {
        for payload in [
            r#"{}"#,
            r#"{"mode":"exclusive"}"#,
            r#"{"mode":"fullscreen","monitor":"other"}"#,
            r#"{"mode":"windowed","bounds":{"width":1}}"#,
            r#"{"mode":true}"#,
        ] {
            let response = JsonValue::parse(
                &host_with_window_fullscreen(RecordingWindowFullscreen::default())
                    .handle_json(&request_v1_21("window.fullscreen.set", payload)),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
        }
    }

    #[test]
    fn window_fullscreen_service_failures_map_to_safe_shared_codes() {
        for (error, code) in [
            (
                WindowFullscreenServiceError::Unavailable,
                "window.unavailable",
            ),
            (WindowFullscreenServiceError::Busy, "window.busy"),
        ] {
            let response = JsonValue::parse(
                &host_with_window_fullscreen(RecordingWindowFullscreen {
                    result: Some(error),
                    ..RecordingWindowFullscreen::default()
                })
                .handle_json(&request_v1_21(
                    "window.fullscreen.set",
                    r#"{"mode":"fullscreen"}"#,
                )),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(code),
                "{error:?} mapped to the wrong code"
            );
        }
    }

    #[test]
    fn a_host_without_a_window_fullscreen_service_reports_unavailable() {
        let response = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowFullscreen],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable(),
            )
            .handle_json(&request_v1_21(
                "window.fullscreen.set",
                r#"{"mode":"windowed"}"#,
            )),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("window.unavailable")
        );
    }

    #[test]
    fn a_granted_window_size_reaches_only_the_size_service() {
        let service = RecordingWindowSize::default();
        let applied = Arc::clone(&service.applied);
        let response = JsonValue::parse(&host_with_window_size(service).handle_json(
            &request_v1_23("window.size.set", r#"{"width":800,"height":600}"#),
        ))
        .expect("response JSON is valid");

        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&response, "result"), "status").as_string(),
            Some("applied")
        );
        assert_eq!(
            applied.lock().expect("the test mutex is usable").as_slice(),
            &[WindowSize::new(800, 600).expect("fixture size is valid")]
        );
    }

    #[test]
    fn window_size_needs_its_own_grant_and_protocol_version() {
        let payload = r#"{"width":800,"height":600}"#;
        let denied = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowFullscreen, Capability::WindowState],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable().with_window_size(RecordingWindowSize::default()),
            )
            .handle_json(&request_v1_23("window.size.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let unsupported = JsonValue::parse(
            &host_with_window_size(RecordingWindowSize::default())
                .handle_json(&request_v1_22("window.size.set", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn window_size_payload_is_exact_and_bounded() {
        for payload in [
            r#"{}"#,
            r#"{"width":319,"height":600}"#,
            r#"{"width":800,"height":239}"#,
            r#"{"width":3841,"height":600}"#,
            r#"{"width":800,"height":2161}"#,
            r#"{"width":800.0,"height":600}"#,
            r#"{"width":800,"height":600,"x":0}"#,
            r#"{"width":800,"height":600,"monitor":"other"}"#,
        ] {
            let response = JsonValue::parse(
                &host_with_window_size(RecordingWindowSize::default())
                    .handle_json(&request_v1_23("window.size.set", payload)),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
        }
    }

    #[test]
    fn window_size_service_failures_map_to_safe_shared_codes() {
        for (error, code) in [
            (WindowSizeServiceError::Unavailable, "window.unavailable"),
            (WindowSizeServiceError::Busy, "window.busy"),
        ] {
            let response = JsonValue::parse(
                &host_with_window_size(RecordingWindowSize {
                    result: Some(error),
                    ..RecordingWindowSize::default()
                })
                .handle_json(&request_v1_23(
                    "window.size.set",
                    r#"{"width":800,"height":600}"#,
                )),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(code),
                "{error:?} mapped to the wrong code"
            );
        }
    }

    #[test]
    fn a_host_without_a_window_size_service_reports_unavailable() {
        let response = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowSize],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable(),
            )
            .handle_json(&request_v1_23(
                "window.size.set",
                r#"{"width":800,"height":600}"#,
            )),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("window.unavailable")
        );
    }

    #[test]
    fn a_granted_complete_menu_reaches_only_the_menu_service() {
        let service = RecordingMenu::default();
        let replacements = Arc::clone(&service.replacements);
        let host = host_with_menu(service);
        let first_payload = r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":true}]}]}"#;
        let first =
            JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", first_payload)))
                .expect("response JSON is valid");
        assert_eq!(field(&first, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&first, "result"), "revision").as_string(),
            Some("1")
        );

        let second = JsonValue::parse(&host.handle_json(&request_v1_18(
            "menu.replace",
            r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":false}]}]}"#,
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&second, "result"), "revision").as_string(),
            Some("2")
        );

        let replacements = replacements
            .lock()
            .expect("menu recorder lock is available");
        assert_eq!(replacements.len(), 2);
        assert_eq!(replacements[0].0.value(), 1);
        assert_eq!(replacements[0].1.menus()[0].label().as_str(), "File");
        assert!(replacements[0].1.menus()[0].items()[0].enabled());
        assert_eq!(replacements[1].0.value(), 2);
        assert!(!replacements[1].1.menus()[0].items()[0].enabled());
    }

    #[test]
    fn a_menu_needs_its_own_grant_protocol_version_and_host_surface() {
        let payload = r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":true}]}]}"#;
        let denied = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::WindowState],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable().with_menu(RecordingMenu::default()),
            )
            .handle_json(&request_v1_18("menu.replace", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let unsupported = JsonValue::parse(
            &host_with_menu(RecordingMenu::default())
                .handle_json(&request_v1_17("menu.replace", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );

        let unavailable = JsonValue::parse(
            &host_with_menu(RecordingMenu::unavailable())
                .handle_json(&request_v1_18("menu.replace", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unavailable, "error"), "code").as_string(),
            Some("menu.unavailable")
        );
    }

    #[test]
    fn a_v1_24_menu_shortcut_is_canonical_unique_and_version_gated() {
        let service = RecordingMenu::default();
        let replacements = Arc::clone(&service.replacements);
        let host = host_with_menu(service);
        let payload = r#"{"menus":[{"label":"File","items":[{"id":"document.complete","label":"Complete","enabled":true,"shortcut":"Ctrl+Shift+M"}]}]}"#;
        let accepted = JsonValue::parse(&host.handle_json(&request_v1_24("menu.replace", payload)))
            .expect("response JSON is valid");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        let replacements = replacements
            .lock()
            .expect("menu recorder lock is available");
        assert_eq!(
            replacements[0].1.menus()[0].items()[0]
                .shortcut()
                .expect("shortcut is retained")
                .display_text(),
            "Ctrl+Shift+M"
        );
        drop(replacements);

        let old_version =
            JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", payload)))
                .expect("response JSON is valid");
        assert_eq!(
            field(field(&old_version, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        for invalid in [
            r#"{"menus":[{"label":"File","items":[{"id":"document.complete","label":"Complete","enabled":true,"shortcut":"Ctrl+m"}]}]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"document.primary","label":"Primary","enabled":true,"shortcut":"Ctrl+M"},{"id":"document.secondary","label":"Secondary","enabled":false,"shortcut":"Ctrl+M"}]}]}"#,
        ] {
            let response =
                JsonValue::parse(&host.handle_json(&request_v1_24("menu.replace", invalid)))
                    .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{invalid} was accepted"
            );
        }
    }

    #[test]
    fn a_menu_payload_is_exact_bounded_and_cannot_name_native_behavior() {
        let service = RecordingMenu::default();
        let replacements = Arc::clone(&service.replacements);
        let host = host_with_menu(service);
        for payload in [
            r#"{}"#,
            r#"{"menus":[]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":"true"}]}]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":true,"nativeId":1}]}]}"#,
            r#"{"menus":[{"label":"File\nOpen","items":[{"id":"document.new","label":"New document","enabled":true}]}]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"native command","label":"New document","enabled":true}]}]}"#,
        ] {
            let response =
                JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", payload)))
                    .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
        }

        let items = (0..16)
            .map(|item| {
                format!(
                    r#"{{"id":"command{item}","label":"{}","enabled":true}}"#,
                    "x".repeat(96)
                )
            })
            .collect::<Vec<_>>();
        let menus = (0..8)
            .map(|menu| format!(r#"{{"label":"Menu{menu}","items":[{}]}}"#, items.join(",")))
            .collect::<Vec<_>>();
        let oversized = format!(r#"{{"menus":[{}]}}"#, menus.join(","));
        assert!(oversized.len() > MAX_MENU_REPLACE_REQUEST_BYTES);
        let response =
            JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", &oversized)))
                .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
        assert!(
            replacements
                .lock()
                .expect("menu recorder lock is available")
                .is_empty()
        );
    }

    /// A reader returning fixed values, standing in for a live surface.
    #[derive(Debug)]
    struct FixedFields {
        result: Result<Vec<(&'static str, &'static str)>, UiFieldReadError>,
    }

    impl UiFieldReader for FixedFields {
        fn read(&self) -> Result<UiFieldSnapshot, UiFieldReadError> {
            let pairs = self.result.as_ref().map_err(|error| *error)?;
            let mut states = anodrel_ui::UiFieldStates::new();
            let children = pairs
                .iter()
                .map(|(id, value)| {
                    anodrel_ui::UiNode::Field(
                        anodrel_ui::Field::new(
                            anodrel_ui::ElementId::new(*id).expect("test ID is valid"),
                            "Label",
                            *value,
                            64,
                            14,
                            true,
                        )
                        .expect("test field is valid"),
                    )
                })
                .collect();
            let document = anodrel_ui::UiDocument::new(anodrel_ui::UiNode::Stack(
                anodrel_ui::Stack::new(
                    anodrel_ui::ElementId::new("root").expect("test ID is valid"),
                    anodrel_ui::Axis::Vertical,
                    anodrel_ui::Insets::zero(),
                    0,
                    children,
                )
                .expect("test stack is valid"),
            ))
            .expect("test document is valid");
            states.reseed(&document);
            UiFieldSnapshot::from_states(&states).map_err(|_| UiFieldReadError::Unavailable)
        }
    }

    fn request_v1_15(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":15}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn host_with_fields(reader: FixedFields) -> CoreHost {
        CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::UiFieldsRead],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_ui_fields(reader),
        )
    }

    #[test]
    fn a_granted_field_read_returns_every_value_at_once() {
        let response = JsonValue::parse(
            &host_with_fields(FixedFields {
                result: Ok(vec![("name", "Ada"), ("city", "London")]),
            })
            .handle_json(&request_v1_15("ui.fields.read", "{}")),
        )
        .expect("response JSON is valid");

        assert_eq!(field(&response, "status").as_string(), Some("success"));
        let JsonValue::Array(fields) = field(field(&response, "result"), "fields") else {
            panic!("the result carries an array of fields");
        };
        let pairs: Vec<(Option<&str>, Option<&str>)> = fields
            .iter()
            .map(|entry| {
                (
                    field(entry, "id").as_string(),
                    field(entry, "value").as_string(),
                )
            })
            .collect();
        // Element-ID order, so the sequence never reports which field was
        // touched last.
        assert_eq!(
            pairs,
            [(Some("city"), Some("London")), (Some("name"), Some("Ada"))]
        );
    }

    #[test]
    fn a_field_read_accepts_no_selector_of_any_kind() {
        // The absence of a selector is the security property: a caller able to
        // narrow a read to one field could repeat it until the typing was
        // reconstructed. See Decision 0067.
        for payload in [
            r#"{"id":"password"}"#,
            r#"{"fields":["password"]}"#,
            r#"{"ids":[]}"#,
            r#"{"since":1}"#,
            r#"{"includeCaret":true}"#,
        ] {
            let response = JsonValue::parse(
                &host_with_fields(FixedFields {
                    result: Ok(vec![("name", "Ada")]),
                })
                .handle_json(&request_v1_15("ui.fields.read", payload)),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
        }
    }

    #[test]
    fn a_field_read_needs_its_own_grant_and_its_own_protocol_version() {
        let denied = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::UiDocumentWrite, Capability::UiEventsRead],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable().with_ui_fields(FixedFields {
                    result: Ok(vec![("name", "Ada")]),
                }),
            )
            .handle_json(&request_v1_15("ui.fields.read", "{}")),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        // Writing a document does not imply reading what was typed into it.
        let unsupported = JsonValue::parse(
            &host_with_fields(FixedFields {
                result: Ok(vec![("name", "Ada")]),
            })
            .handle_json(&request_v1_14("ui.fields.read", "{}")),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn a_host_without_a_surface_reports_one_unavailable_code() {
        for host in [
            host_with_fields(FixedFields {
                result: Err(UiFieldReadError::Unavailable),
            }),
            // No reader supplied at all takes the same path, so an application
            // cannot tell a host without fields from one that refused.
            CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::UiFieldsRead],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable(),
            ),
        ] {
            let response =
                JsonValue::parse(&host.handle_json(&request_v1_15("ui.fields.read", "{}")))
                    .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("ui.fields.unavailable")
            );
        }
    }

    #[test]
    fn no_operation_or_capability_reaches_a_host_crash_record() {
        // Crash records are host-only by design: the diagnostic ledger is
        // readable behind `diagnostics.read`, and a crash record is readable
        // through nothing at all. Merging the two would put host defect
        // information behind a grant an application can hold. This is the
        // invariant most likely to be broken by someone adding a convenience,
        // so it is asserted rather than left to the absence of code.
        // See docs/CRASH_REPORTS.md and Decision 0065.
        for operation in [
            "crash.read",
            "crash.records.read",
            "crash.report",
            "diagnostics.crash.read",
            "host.crash.read",
        ] {
            let response = JsonValue::parse(
                &host_with_notifications(RecordingNotifications::default())
                    .handle_json(&request_v1_13(operation, "{}")),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("operation.unsupported"),
                "{operation} was answered by this host"
            );
        }

        for capability in [
            Capability::DiagnosticsRead,
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::SessionClose,
            Capability::ClipboardRead,
            Capability::ClipboardWrite,
            Capability::ExternalOpen,
            Capability::NetworkFetch,
            Capability::DialogOpenFile,
            Capability::DialogSaveFile,
            Capability::FileReadText,
            Capability::StorageStateRead,
            Capability::StorageStateReplace,
            Capability::StorageStateClear,
            Capability::CredentialRead,
            Capability::CredentialWrite,
            Capability::CredentialDelete,
            Capability::NotificationShow,
        ] {
            assert!(
                !capability.as_str().contains("crash"),
                "{} names a crash surface",
                capability.as_str()
            );
        }
    }

    #[test]
    fn a_granted_notification_reaches_the_service_unchanged() {
        let service = RecordingNotifications::default();
        let response = JsonValue::parse(&host_with_notifications(service).handle_json(
            &request_v1_13(
                "notification.show",
                &notification_payload("Done", "All green"),
            ),
        ))
        .expect("response JSON is valid");

        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&response, "result"), "status").as_string(),
            Some("shown")
        );
    }

    #[test]
    fn a_rejected_notification_never_echoes_its_own_text_back() {
        // A refusal must not become a way to have the host repeat content.
        let response = JsonValue::parse(
            &host_with_notifications(RecordingNotifications::default()).handle_json(
                &request_v1_13(
                    "notification.show",
                    &notification_payload("Spoofed\rsecond line", "body"),
                ),
            ),
        )
        .expect("response JSON is valid");

        let error = field(&response, "error");
        assert_eq!(
            field(error, "code").as_string(),
            Some("notification.text_invalid")
        );
        assert!(!response.to_json().contains("Spoofed"));
    }

    #[test]
    fn notification_payloads_accept_exactly_a_title_and_a_body() {
        let host = host_with_notifications(RecordingNotifications::default());
        for payload in [
            r#"{"title":"only"}"#,
            r#"{"body":"only"}"#,
            // An extra field is a mismatch, not something to ignore, so a
            // future urgency or action field cannot be smuggled past 1.13.
            r#"{"title":"a","body":"b","urgency":"high"}"#,
            r#"{"title":"a","body":2}"#,
        ] {
            let response =
                JsonValue::parse(&host.handle_json(&request_v1_13("notification.show", payload)))
                    .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
        }
    }

    #[test]
    fn service_failures_stay_distinguishable_without_describing_the_user() {
        for (error, expected) in [
            (
                NotificationServiceError::Unavailable,
                "notification.unavailable",
            ),
            (NotificationServiceError::Busy, "notification.busy"),
        ] {
            let response = JsonValue::parse(
                &host_with_notifications(RecordingNotifications::failing(error)).handle_json(
                    &request_v1_13(
                        "notification.show",
                        &notification_payload("Done", "All green"),
                    ),
                ),
            )
            .expect("response JSON is valid");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(expected)
            );
        }
    }

    #[test]
    fn accepts_ping_and_formats_a_utc_timestamp() {
        let response = JsonValue::parse(&host(vec![]).handle_json(&request(
            "platform.ping",
            r#"{"sentAt":"2026-07-31T12:00:00.000Z"}"#,
        )))
        .expect("response JSON is valid");
        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert!(
            field(field(&response, "result"), "receivedAt")
                .as_string()
                .is_some_and(|timestamp| timestamp.ends_with('Z'))
        );
    }

    #[test]
    fn rejects_forged_capability_context() {
        let response = JsonValue::parse(&host(vec![]).handle_json(&format!(
            r#"{},"capabilityContext":{{"grantedCapabilities":["diagnostics.read"]}}}}"#,
            request("platform.health", "{}")
                .strip_suffix('}')
                .expect("request ends with a brace")
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("capability.denied")
        );
    }

    #[test]
    fn replaces_ui_documents_only_with_the_current_capability_and_protocol_minor() {
        let document = valid_ui_document("Continue");
        let update_request = request_v1_1("ui.document.replace", &ui_document_payload(&document));

        let denied = JsonValue::parse(&host(vec![]).handle_json(&update_request))
            .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let host = host(vec![Capability::UiDocumentWrite]);
        let first =
            JsonValue::parse(&host.handle_json(&update_request)).expect("response JSON is valid");
        assert_eq!(field(&first, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&first, "result"), "revision").as_string(),
            Some("1")
        );
        let first_snapshot = host
            .take_ui_document_update()
            .expect("accepted document is available to the transport");
        assert_eq!(first_snapshot.revision().value(), 1);
        assert_eq!(first_snapshot.document().root().id().as_str(), "root");
        assert!(host.take_ui_document_update().is_none());

        let invalid = request_v1_1("ui.document.replace", &ui_document_payload("not JSON"));
        let invalid =
            JsonValue::parse(&host.handle_json(&invalid)).expect("response JSON is valid");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let second_document = valid_ui_document("Continue safely");
        let second = JsonValue::parse(&host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&second_document),
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&second, "result"), "revision").as_string(),
            Some("2")
        );

        let old_minor = JsonValue::parse(&host.handle_json(&request(
            "ui.document.replace",
            &ui_document_payload(&document),
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&old_minor, "error"), "code").as_string(),
            Some("operation.unsupported")
        );

        let oversized = request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&"x".repeat(MAX_UI_DOCUMENT_REQUEST_BYTES + 1)),
        );
        let oversized =
            JsonValue::parse(&host.handle_json(&oversized)).expect("response JSON is valid");
        assert_eq!(
            field(field(&oversized, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn replaces_version_two_documents_only_through_the_new_operation() {
        let host = host(vec![Capability::UiDocumentWrite]);
        let document = valid_ui_document_v2();

        let accepted = JsonValue::parse(&host.handle_json(&request_v1_4(
            "ui.document.replace.v2",
            &ui_document_payload(document),
        )))
        .expect("response JSON is valid");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        let snapshot = host
            .take_ui_document_update()
            .expect("accepted version two document is delivered");
        assert_eq!(snapshot.document().root().id().as_str(), "viewport");

        let wrong_operation = JsonValue::parse(&host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(document),
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&wrong_operation, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn replaces_version_three_status_documents_only_through_protocol_1_26() {
        let host = host(vec![Capability::UiDocumentWrite]);
        let document = valid_ui_document_v3("Saved", "polite");

        let accepted = JsonValue::parse(&host.handle_json(&request_v1_26(
            "ui.document.replace.v3",
            &ui_document_payload(&document),
        )))
        .expect("response JSON is valid");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        let snapshot = host
            .take_ui_document_update()
            .expect("accepted version three document is delivered");
        assert_eq!(
            snapshot.document().status().map(|status| status.value()),
            Some("Saved")
        );

        let wrong_operation = JsonValue::parse(&host.handle_json(&request_v1_4(
            "ui.document.replace.v2",
            &ui_document_payload(&document),
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&wrong_operation, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let old_minor = JsonValue::parse(&host.handle_json(&request_v1_25(
            "ui.document.replace.v3",
            &ui_document_payload(&document),
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&old_minor, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn reads_only_current_enabled_ui_actions_from_the_supplied_input_mailbox() {
        let mailbox = UiInputMailbox::new();
        let host = CoreHost::with_ui_input_mailbox(
            HostPolicy::new(
                "test.application",
                vec![Capability::UiDocumentWrite, Capability::UiEventsRead],
                "test-host",
            )
            .expect("test policy is valid"),
            mailbox.clone(),
        );
        let document = valid_ui_document("Continue");
        let update = JsonValue::parse(&host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&document),
        )))
        .expect("update response is JSON");
        assert_eq!(
            field(field(&update, "result"), "revision").as_string(),
            Some("1")
        );

        let current = host
            .take_ui_document_update()
            .expect("accepted document is available")
            .revision();
        let action = UiEvent::ActionInvoked(ElementId::new("root").expect("test ID is valid"));
        mailbox.push(UiInputCandidate::new(current, action.clone()));
        let read = JsonValue::parse(&host.handle_json(&request_v1_2("ui.events.read", "{}")))
            .expect("event response is JSON");
        let result = field(&read, "result");
        assert_eq!(field(result, "dropped"), &JsonValue::Number("0".to_owned()));
        assert_eq!(
            field(result, "discarded"),
            &JsonValue::Number("0".to_owned())
        );
        let JsonValue::Array(events) = field(result, "events") else {
            panic!("events is an array");
        };
        assert_eq!(events.len(), 1);
        assert_eq!(
            field(&events[0], "eventName").as_string(),
            Some("ui.action.invoked")
        );
        assert_eq!(
            field(field(&events[0], "payload"), "action").as_string(),
            Some("root")
        );

        let replacement = valid_ui_document("Continue safely");
        let _ = host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&replacement),
        ));
        mailbox.push(UiInputCandidate::new(current, action));
        let stale = JsonValue::parse(&host.handle_json(&request_v1_2("ui.events.read", "{}")))
            .expect("stale event response is JSON");
        let JsonValue::Array(events) = field(field(&stale, "result"), "events") else {
            panic!("events is an array");
        };
        assert!(events.is_empty());
        assert_eq!(
            field(field(&stale, "result"), "discarded"),
            &JsonValue::Number("1".to_owned())
        );
    }

    #[test]
    fn grouped_primary_operations_reuse_the_primary_mailboxes_and_leave_secondary_input_local() {
        let document_mailbox = UiDocumentMailbox::new();
        let input_mailbox = UiInputMailbox::new();
        let group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
            document_mailbox.clone(),
            input_mailbox.clone(),
        );
        let host = CoreHost::with_session_window_group_and_service_bundle(
            HostPolicy::new(
                "test.application",
                vec![Capability::UiDocumentWrite, Capability::UiEventsRead],
                "test-host",
            )
            .expect("test policy is valid"),
            group.clone(),
            SessionCloseSignal::default(),
            HostServices::unavailable(),
        );
        let document = valid_ui_document("Continue");

        let replacement = JsonValue::parse(&host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&document),
        )))
        .expect("replacement response is JSON");
        assert_eq!(field(&replacement, "status").as_string(), Some("success"));
        assert!(
            host.take_ui_document_update().is_none(),
            "the group publishes directly to its primary mailbox"
        );
        let primary_snapshot = document_mailbox
            .take()
            .expect("the supplied primary mailbox receives the snapshot");
        assert_eq!(primary_snapshot.revision().value(), 1);

        let opening_group = group.clone();
        let opening_document = document.clone();
        let opening = thread::spawn(move || {
            opening_group.open_secondary(
                WindowTitleProposal::new("Secondary").expect("test title is valid"),
                &opening_document,
            )
        });
        let request = loop {
            if let Some(request) = group.take_open_request() {
                break request;
            }
            thread::yield_now();
        };
        assert!(group.complete_open(request.id(), true));
        let secondary = opening
            .join()
            .expect("opening worker does not panic")
            .expect("secondary opens");
        let secondary_resources = group
            .resources(&secondary)
            .expect("secondary resources are registered");
        secondary_resources
            .input_mailbox()
            .push(UiInputCandidate::new(
                request.snapshot().snapshot().revision(),
                UiEvent::ActionInvoked(ElementId::new("root").expect("test ID is valid")),
            ));

        let primary_read =
            JsonValue::parse(&host.handle_json(&request_v1_2("ui.events.read", "{}")))
                .expect("event response is JSON");
        let JsonValue::Array(events) = field(field(&primary_read, "result"), "events") else {
            panic!("events is an array");
        };
        assert!(events.is_empty());
        assert_eq!(
            group
                .drain_input_batch(&secondary)
                .expect("secondary remains registered")
                .into_candidates()
                .len(),
            1,
            "targetless primary reads cannot consume a secondary view's input"
        );
    }

    #[test]
    fn protocol_v1_25_opens_targets_reads_and_closes_only_session_owned_views() {
        let document_mailbox = UiDocumentMailbox::new();
        let input_mailbox = UiInputMailbox::new();
        let group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
            document_mailbox,
            input_mailbox,
        );
        let host = CoreHost::with_session_window_group_and_service_bundle(
            HostPolicy::new(
                "test.application",
                vec![
                    Capability::WindowOpen,
                    Capability::WindowClose,
                    Capability::UiDocumentWrite,
                    Capability::UiEventsRead,
                ],
                "test-host",
            )
            .expect("test policy is valid"),
            group.clone(),
            SessionCloseSignal::default(),
            HostServices::unavailable(),
        );
        let document = valid_ui_document("Secondary action");
        let opening_group = group.clone();
        let native_creator = thread::spawn(move || {
            loop {
                if let Some(request) = opening_group.take_open_request() {
                    assert_eq!(request.context().as_str(), "Notes");
                    assert!(opening_group.complete_open(request.id(), true));
                    break;
                }
                thread::yield_now();
            }
        });

        let open_payload = object([
            ("document", JsonValue::String(document.clone())),
            ("title", JsonValue::String("Notes".to_owned())),
        ])
        .to_json();
        let opened =
            JsonValue::parse(&host.handle_json(&request_v1_25("window.open", &open_payload)))
                .expect("open response is JSON");
        native_creator
            .join()
            .expect("native group creator does not panic");
        assert_eq!(field(&opened, "status").as_string(), Some("success"));
        let window_id = field(field(&opened, "result"), "windowId")
            .as_string()
            .expect("open result carries an identity");
        assert_eq!(window_id, "window-1");
        let secondary = UiWindowId::parse(window_id).expect("fixed secondary ID parses");

        let replacement_payload = object([
            ("document", JsonValue::String(document.clone())),
            ("windowId", JsonValue::String(window_id.to_owned())),
        ])
        .to_json();
        let replacement = JsonValue::parse(&host.handle_json(&request_v1_25(
            "ui.document.replace.window",
            &replacement_payload,
        )))
        .expect("replacement response is JSON");
        assert_eq!(
            field(field(&replacement, "result"), "revision").as_string(),
            Some("2")
        );

        let secondary_resources = group
            .resources(&secondary)
            .expect("secondary resources remain available");
        let revision = secondary_resources
            .document_mailbox()
            .take()
            .expect("targeted replacement publishes the secondary snapshot")
            .revision();
        secondary_resources
            .input_mailbox()
            .push(UiInputCandidate::new(
                revision,
                UiEvent::ActionInvoked(ElementId::new("root").expect("fixed action ID is valid")),
            ));
        let events =
            JsonValue::parse(&host.handle_json(&request_v1_25("ui.events.read.window", "{}")))
                .expect("events response is JSON");
        let JsonValue::Array(events) = field(field(&events, "result"), "events") else {
            panic!("events result is an array");
        };
        assert_eq!(events.len(), 1);
        assert_eq!(field(&events[0], "windowId").as_string(), Some("window-1"));
        assert_eq!(
            field(&events[0], "eventName").as_string(),
            Some("ui.action.invoked")
        );

        let close_payload =
            object([("windowId", JsonValue::String(window_id.to_owned()))]).to_json();
        let close =
            JsonValue::parse(&host.handle_json(&request_v1_25("window.close", &close_payload)))
                .expect("close response is JSON");
        assert_eq!(
            field(field(&close, "result"), "status").as_string(),
            Some("requested")
        );
        assert_eq!(
            group.take_secondary_close_requests(),
            vec![secondary.clone()]
        );
        assert!(group.close_secondary(&secondary).is_ok());

        let unavailable = JsonValue::parse(&host.handle_json(&request_v1_25(
            "ui.document.replace.window",
            &replacement_payload,
        )))
        .expect("unavailable response is JSON");
        assert_eq!(
            field(field(&unavailable, "error"), "code").as_string(),
            Some("window.unavailable")
        );
    }

    #[test]
    fn protocol_v1_26_keeps_status_documents_explicit_for_secondary_views() {
        let document_mailbox = UiDocumentMailbox::new();
        let input_mailbox = UiInputMailbox::new();
        let group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
            document_mailbox,
            input_mailbox,
        );
        let host = CoreHost::with_session_window_group_and_service_bundle(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowOpen, Capability::UiDocumentWrite],
                "test-host",
            )
            .expect("test policy is valid"),
            group.clone(),
            SessionCloseSignal::default(),
            HostServices::unavailable(),
        );
        let initial = valid_ui_document_v3("Saved", "polite");
        let opening_group = group.clone();
        let native_creator = thread::spawn(move || {
            loop {
                if let Some(request) = opening_group.take_open_request() {
                    assert!(opening_group.complete_open(request.id(), true));
                    break;
                }
                thread::yield_now();
            }
        });

        let opened = JsonValue::parse(
            &host.handle_json(&request_v1_26(
                "window.open.v3",
                &object([
                    ("document", JsonValue::String(initial.clone())),
                    ("title", JsonValue::String("Status".to_owned())),
                ])
                .to_json(),
            )),
        )
        .expect("open response is JSON");
        native_creator
            .join()
            .expect("native group creator does not panic");
        let window_id = field(field(&opened, "result"), "windowId")
            .as_string()
            .expect("open result carries an identity");
        let secondary = UiWindowId::parse(window_id).expect("fixed secondary ID parses");
        let resources = group
            .resources(&secondary)
            .expect("secondary is registered");
        let initial_snapshot = resources
            .document_mailbox()
            .take()
            .expect("initial v3 snapshot is published");
        assert_eq!(
            initial_snapshot
                .document()
                .status()
                .map(|status| status.value()),
            Some("Saved")
        );

        let updated = valid_ui_document_v3("Save failed", "assertive");
        let replacement = JsonValue::parse(
            &host.handle_json(&request_v1_26(
                "ui.document.replace.window.v3",
                &object([
                    ("document", JsonValue::String(updated)),
                    ("windowId", JsonValue::String(window_id.to_owned())),
                ])
                .to_json(),
            )),
        )
        .expect("replacement response is JSON");
        assert_eq!(
            field(field(&replacement, "result"), "revision").as_string(),
            Some("2")
        );
        let replacement_snapshot = resources
            .document_mailbox()
            .take()
            .expect("updated v3 snapshot is published");
        assert_eq!(
            replacement_snapshot
                .document()
                .status()
                .map(|status| status.value()),
            Some("Save failed")
        );

        let v1_refusal = JsonValue::parse(
            &host.handle_json(&request_v1_25(
                "window.open.v3",
                &object([
                    ("document", JsonValue::String(initial)),
                    ("title", JsonValue::String("Status".to_owned())),
                ])
                .to_json(),
            )),
        )
        .expect("old-version response is JSON");
        assert_eq!(
            field(field(&v1_refusal, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn menu_and_document_actions_share_ordered_revision_checked_delivery() {
        let mailbox = UiInputMailbox::new();
        let host = CoreHost::with_session_components_and_service_bundle(
            HostPolicy::new(
                "test.application",
                vec![
                    Capability::UiDocumentWrite,
                    Capability::UiEventsRead,
                    Capability::MenuWrite,
                ],
                "test-host",
            )
            .expect("test policy is valid"),
            mailbox.clone(),
            SessionCloseSignal::default(),
            HostServices::unavailable().with_menu(RecordingMenu::default()),
        );
        let document = valid_ui_document("Continue");
        let document_response = JsonValue::parse(&host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&document),
        )))
        .expect("document response is JSON");
        assert_eq!(
            field(field(&document_response, "result"), "revision").as_string(),
            Some("1")
        );
        let document_revision = host
            .take_ui_document_update()
            .expect("accepted document is available")
            .revision();

        let menu_payload = r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":true}]}]}"#;
        let menu_response =
            JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", menu_payload)))
                .expect("menu response is JSON");
        assert_eq!(
            field(field(&menu_response, "result"), "revision").as_string(),
            Some("1")
        );
        let menu_revision = anodrel_menu::MenuRevision::INITIAL
            .next()
            .expect("first menu revision exists");
        let menu_action =
            anodrel_menu::MenuActionId::new("document.new").expect("test menu action is valid");

        mailbox.push(UiInputCandidate::new(
            document_revision,
            UiEvent::ActionInvoked(ElementId::new("root").expect("test ID is valid")),
        ));
        mailbox.push(MenuInputCandidate::new(menu_revision, menu_action.clone()));
        let read = JsonValue::parse(&host.handle_json(&request_v1_18("ui.events.read", "{}")))
            .expect("event response is JSON");
        let result = field(&read, "result");
        assert_eq!(field(result, "dropped"), &JsonValue::Number("0".to_owned()));
        assert_eq!(
            field(result, "discarded"),
            &JsonValue::Number("0".to_owned())
        );
        let JsonValue::Array(events) = field(result, "events") else {
            panic!("events is an array");
        };
        assert_eq!(events.len(), 2);
        assert_eq!(
            field(&events[0], "eventName").as_string(),
            Some("ui.action.invoked")
        );
        assert_eq!(
            field(&events[1], "eventName").as_string(),
            Some("menu.action.invoked")
        );
        assert_eq!(field(&events[1], "source").as_string(), Some("native.menu"));
        assert_eq!(
            field(field(&events[1], "schemaVersion"), "minor"),
            &JsonValue::Number("18".to_owned())
        );
        assert_eq!(
            field(field(&events[1], "payload"), "menuRevision").as_string(),
            Some("1")
        );
        assert_eq!(
            field(field(&events[1], "payload"), "action").as_string(),
            Some("document.new")
        );

        let disabled = r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":false}]}]}"#;
        let _ = host.handle_json(&request_v1_18("menu.replace", disabled));
        mailbox.push(MenuInputCandidate::new(menu_revision, menu_action));
        let stale = JsonValue::parse(&host.handle_json(&request_v1_18("ui.events.read", "{}")))
            .expect("stale event response is JSON");
        let JsonValue::Array(events) = field(field(&stale, "result"), "events") else {
            panic!("events is an array");
        };
        assert!(events.is_empty());
        assert_eq!(
            field(field(&stale, "result"), "discarded"),
            &JsonValue::Number("1".to_owned())
        );
    }

    #[test]
    fn accepts_only_a_granted_current_protocol_session_close_request() {
        let signal = SessionCloseSignal::default();
        let close_host = CoreHost::with_session_components(
            HostPolicy::new(
                "test.application",
                vec![Capability::SessionClose],
                "test-host",
            )
            .expect("test policy is valid"),
            UiInputMailbox::new(),
            signal.clone(),
        );
        let accepted =
            JsonValue::parse(&close_host.handle_json(&request_v1_3("session.close", "{}")))
                .expect("response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("accepted")
        );
        assert!(signal.take());
        assert!(!signal.take());

        let denied =
            JsonValue::parse(&host(vec![]).handle_json(&request_v1_3("session.close", "{}")))
                .expect("response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let old_minor =
            JsonValue::parse(&close_host.handle_json(&request_v1_2("session.close", "{}")))
                .expect("response is JSON");
        assert_eq!(
            field(field(&old_minor, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn clipboard_operations_are_separate_bounded_and_capability_checked() {
        let clipboard_host = clipboard_host(
            vec![Capability::ClipboardRead, Capability::ClipboardWrite],
            MemoryClipboard::with_text(Some("before")),
        );

        let read =
            JsonValue::parse(&clipboard_host.handle_json(&request_v1_5("clipboard.read", "{}")))
                .expect("clipboard read response is JSON");
        assert_eq!(field(&read, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&read, "result"), "status").as_string(),
            Some("text")
        );
        assert_eq!(
            field(field(&read, "result"), "text").as_string(),
            Some("before")
        );

        let write = JsonValue::parse(
            &clipboard_host.handle_json(&request_v1_5("clipboard.write", r#"{"text":"after"}"#)),
        )
        .expect("clipboard write response is JSON");
        assert_eq!(
            field(field(&write, "result"), "status").as_string(),
            Some("written")
        );

        let updated =
            JsonValue::parse(&clipboard_host.handle_json(&request_v1_5("clipboard.read", "{}")))
                .expect("updated clipboard read response is JSON");
        assert_eq!(
            field(field(&updated, "result"), "text").as_string(),
            Some("after")
        );

        let denied =
            JsonValue::parse(&host(vec![]).handle_json(&request_v1_5("clipboard.read", "{}")))
                .expect("denied clipboard response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let oversized = object([(
            "text",
            JsonValue::String("x".repeat(MAX_CLIPBOARD_TEXT_REQUEST_BYTES + 1)),
        )])
        .to_json();
        let rejected = JsonValue::parse(
            &clipboard_host.handle_json(&request_v1_5("clipboard.write", &oversized)),
        )
        .expect("oversized clipboard response is JSON");
        assert_eq!(
            field(field(&rejected, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn clipboard_service_failures_have_safe_stable_protocol_codes() {
        let host = clipboard_host(
            vec![Capability::ClipboardRead, Capability::ClipboardWrite],
            FailingClipboard(ClipboardServiceError::StoredTextInvalid),
        );
        let response = JsonValue::parse(&host.handle_json(&request_v1_5("clipboard.read", "{}")))
            .expect("clipboard failure response is JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("clipboard.text_invalid")
        );
        assert!(
            field(field(&response, "error"), "message")
                .as_string()
                .is_some_and(|message| !message.contains("before"))
        );
    }

    #[test]
    fn external_open_requires_its_own_grant_and_validated_https_url() {
        let external_host = external_host(
            vec![Capability::ExternalOpen],
            RecordingExternalLinks::default(),
        );
        let accepted = JsonValue::parse(&external_host.handle_json(&request_v1_6(
            "external.open",
            r#"{"url":"https://docs.anodrel.dev/guide"}"#,
        )))
        .expect("external open response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("opened")
        );

        let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_6(
            "external.open",
            r#"{"url":"https://docs.anodrel.dev/guide"}"#,
        )))
        .expect("denied external open response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&external_host.handle_json(&request_v1_6(
            "external.open",
            r#"{"url":"file:///C:/private.txt"}"#,
        )))
        .expect("invalid external open response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn external_service_failure_never_exposes_a_url_or_native_status() {
        let host = external_host(vec![Capability::ExternalOpen], FailingExternalLinks);
        let response = JsonValue::parse(&host.handle_json(&request_v1_6(
            "external.open",
            r#"{"url":"https://docs.anodrel.dev/private"}"#,
        )))
        .expect("external failure response is JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("external.unavailable")
        );
        assert!(
            field(field(&response, "error"), "message")
                .as_string()
                .is_some_and(|message| !message.contains("private"))
        );
    }

    #[test]
    fn network_text_fetch_is_separately_granted_and_returns_only_status_and_text() {
        let service = RecordingNetwork::responding(201, "created");
        let requested = Arc::clone(&service.requested);
        let host = network_host(vec![Capability::NetworkFetch], service);
        let response = JsonValue::parse(&host.handle_json(&request_v1_19(
            "network.fetch_text",
            r#"{"url":"https://Api.Example.test:8443/v1/status?format=text"}"#,
        )))
        .expect("network response is JSON");
        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&response, "result"), "statusCode"),
            &JsonValue::Number("201".to_owned())
        );
        assert_eq!(
            field(field(&response, "result"), "text").as_string(),
            Some("created")
        );
        let requested = requested
            .lock()
            .expect("network recorder lock is available");
        assert_eq!(requested.len(), 1);
        assert_eq!(
            requested[0].as_str(),
            "https://Api.Example.test:8443/v1/status?format=text"
        );
        assert_eq!(requested[0].hostname(), "api.example.test");
        assert_eq!(requested[0].port(), 8443);
    }

    #[test]
    fn network_text_fetch_requires_its_protocol_version_grant_and_host_service() {
        let payload = r#"{"url":"https://api.example.test/status"}"#;
        let denied = JsonValue::parse(
            &network_host(vec![], RecordingNetwork::responding(200, "healthy"))
                .handle_json(&request_v1_19("network.fetch_text", payload)),
        )
        .expect("denied network response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );
        assert_eq!(
            field(field(&denied, "error"), "details")
                .as_object()
                .and_then(|details| details.get("capability"))
                .and_then(JsonValue::as_string),
            Some("network.fetch")
        );

        let unsupported = JsonValue::parse(
            &network_host(
                vec![Capability::NetworkFetch],
                RecordingNetwork::responding(200, "healthy"),
            )
            .handle_json(&request_v1_18("network.fetch_text", payload)),
        )
        .expect("old-version network response is JSON");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );

        let unavailable = JsonValue::parse(
            &CoreHost::with_services(
                HostPolicy::new(
                    "test.application",
                    vec![Capability::NetworkFetch],
                    "test-host",
                )
                .expect("test policy is valid"),
                HostServices::unavailable(),
            )
            .handle_json(&request_v1_19("network.fetch_text", payload)),
        )
        .expect("unavailable network response is JSON");
        assert_eq!(
            field(field(&unavailable, "error"), "code").as_string(),
            Some("network.unavailable")
        );
    }

    #[test]
    fn network_text_fetch_rejects_unrepresentable_values_and_never_echoes_them() {
        let service = RecordingNetwork::responding(200, "healthy");
        let requested = Arc::clone(&service.requested);
        let host = network_host(vec![Capability::NetworkFetch], service);
        let marker = "PrivateNetworkMarker";
        for payload in [
            r#"{}"#,
            r#"{"url":"https://api.example.test/status","header":"secret"}"#,
            r#"{"url":"https://127.0.0.1/status"}"#,
            &format!(r#"{{"url":"https://api.example.test/{marker}%1"}}"#),
        ] {
            let response = host.handle_json(&request_v1_19("network.fetch_text", payload));
            let parsed = JsonValue::parse(&response).expect("invalid network response is JSON");
            assert_eq!(
                field(field(&parsed, "error"), "code").as_string(),
                Some("request.payload_invalid"),
                "{payload} was accepted"
            );
            assert!(
                !response.contains(marker),
                "refused URL leaked into the response"
            );
        }
        assert!(
            requested
                .lock()
                .expect("network recorder lock is available")
                .is_empty(),
            "a rejected request reached the service"
        );

        for (error, code) in [
            (NetworkTextServiceError::Unavailable, "network.unavailable"),
            (
                NetworkTextServiceError::ResponseInvalid,
                "network.response_invalid",
            ),
        ] {
            let response = JsonValue::parse(
                &network_host(
                    vec![Capability::NetworkFetch],
                    RecordingNetwork::failing(error),
                )
                .handle_json(&request_v1_19(
                    "network.fetch_text",
                    r#"{"url":"https://api.example.test/status"}"#,
                )),
            )
            .expect("failed network response is JSON");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(code),
                "{error:?} mapped to the wrong protocol error"
            );
        }
    }

    #[test]
    fn file_dialog_requires_its_own_grant_and_returns_only_cancellation_or_a_path() {
        let accepted_host =
            file_dialog_host(vec![Capability::DialogOpenFile], CancellingFileDialog);
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
            "dialog.open_file",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("dialog response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("cancelled")
        );

        let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_7(
            "dialog.open_file",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("denied dialog response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
            "dialog.open_file",
            r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
        )))
        .expect("invalid dialog response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn save_dialog_requires_its_own_grant_and_returns_only_cancellation_or_a_destination() {
        let accepted_host = file_dialog_host(vec![Capability::DialogSaveFile], SavingFileDialog);
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_8(
            "dialog.save_file",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("save dialog response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("saved")
        );
        assert!(
            field(field(&accepted, "result"), "path")
                .as_string()
                .is_some_and(|path| !path.is_empty())
        );

        let denied = JsonValue::parse(
            &file_dialog_host(vec![Capability::DialogOpenFile], SavingFileDialog).handle_json(
                &request_v1_8(
                    "dialog.save_file",
                    r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
                ),
            ),
        )
        .expect("denied save dialog response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_8(
            "dialog.save_file",
            r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
        )))
        .expect("invalid save dialog response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let unsupported = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
            "dialog.save_file",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("unsupported save dialog response is JSON");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn save_selection_requires_the_save_grant_and_returns_an_opaque_save_reference() {
        let accepted_host = file_write_host(
            vec![Capability::DialogSaveFile],
            CapturingSaveDialog,
            RecordingFileTextWrite::accepting(),
        );
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_17(
            "dialog.save_file.v2",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("save selection response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "saveReference").as_string(),
            Some("ZyXwVuTsRqPoNmLkJiHgFe")
        );

        let denied = JsonValue::parse(
            &file_write_host(
                vec![Capability::DialogOpenFile],
                CapturingSaveDialog,
                RecordingFileTextWrite::accepting(),
            )
            .handle_json(&request_v1_17(
                "dialog.save_file.v2",
                r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
            )),
        )
        .expect("denied response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_17(
            "dialog.save_file.v2",
            r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
        )))
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let unsupported = JsonValue::parse(&accepted_host.handle_json(&request_v1_16(
            "dialog.save_file.v2",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("unsupported response is JSON");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn selected_output_text_is_separately_granted_bounded_and_safe() {
        let writer = RecordingFileTextWrite::accepting();
        let writes = Arc::clone(&writer.writes);
        let write_host =
            file_write_host(vec![Capability::FileWriteText], CapturingSaveDialog, writer);
        let reference = "ZyXwVuTsRqPoNmLkJiHgFe";
        let accepted = JsonValue::parse(&write_host.handle_json(&request_v1_17(
            "file.write_text",
            &format!(r#"{{"saveReference":"{reference}","text":"selected text"}}"#),
        )))
        .expect("write response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("written")
        );
        assert_eq!(
            writes
                .lock()
                .expect("write recorder lock is available")
                .as_slice(),
            ["selected text"]
        );

        let denied = JsonValue::parse(
            &file_write_host(
                vec![Capability::DialogSaveFile],
                CapturingSaveDialog,
                RecordingFileTextWrite::accepting(),
            )
            .handle_json(&request_v1_17(
                "file.write_text",
                &format!(r#"{{"saveReference":"{reference}","text":"selected text"}}"#),
            )),
        )
        .expect("denied response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&write_host.handle_json(&request_v1_17(
            "file.write_text",
            r#"{"selectionReference":"AbCdEfGhIjKlMnOpQrStUv","text":"selected text"}"#,
        )))
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let oversized = JsonValue::parse(&write_host.handle_json(&request_v1_17(
            "file.write_text",
            &format!(
                r#"{{"saveReference":"{reference}","text":"{}"}}"#,
                "x".repeat(MAX_FILE_TEXT_WRITE_BYTES + 1)
            ),
        )))
        .expect("oversized response is JSON");
        assert_eq!(
            field(field(&oversized, "error"), "code").as_string(),
            Some("file.text_too_large")
        );
        assert_eq!(
            writes
                .lock()
                .expect("write recorder lock is available")
                .as_slice(),
            ["selected text"]
        );

        let unavailable = JsonValue::parse(
            &file_write_host(
                vec![Capability::FileWriteText],
                CapturingSaveDialog,
                RecordingFileTextWrite::failing(FileTextWriteServiceError::Unavailable),
            )
            .handle_json(&request_v1_17(
                "file.write_text",
                &format!(r#"{{"saveReference":"{reference}","text":"private text"}}"#),
            )),
        )
        .expect("unavailable response is JSON");
        assert_eq!(
            field(field(&unavailable, "error"), "code").as_string(),
            Some("file.unavailable")
        );
        assert!(
            field(field(&unavailable, "error"), "message")
                .as_string()
                .is_some_and(|message| !message.contains("private"))
        );

        let unsupported = JsonValue::parse(&write_host.handle_json(&request_v1_16(
            "file.write_text",
            &format!(r#"{{"saveReference":"{reference}","text":"selected text"}}"#),
        )))
        .expect("unsupported response is JSON");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn selected_output_binary_is_canonical_bounded_and_separately_granted() {
        let reference = "ZyXwVuTsRqPoNmLkJiHgFe";
        let writer = RecordingFileBinaryWrite::accepting();
        let writes = Arc::clone(&writer.writes);
        let accepted_host = file_binary_write_host(vec![Capability::FileWriteBinary], writer);
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_22(
            "file.write_binary",
            &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AAEC_w"}}"#),
        )))
        .expect("binary response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("written")
        );
        assert_eq!(
            writes
                .lock()
                .expect("binary-write recorder lock is available")
                .as_slice(),
            [vec![0, 1, 2, 255]]
        );

        let denied_writer = RecordingFileBinaryWrite::accepting();
        let denied_writes = Arc::clone(&denied_writer.writes);
        let denied_discards = Arc::clone(&denied_writer.discarded);
        let denied = JsonValue::parse(
            &file_binary_write_host(vec![Capability::DialogSaveFile], denied_writer).handle_json(
                &request_v1_22(
                    "file.write_binary",
                    &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AB"}}"#),
                ),
            ),
        )
        .expect("denied response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );
        assert!(
            denied_writes
                .lock()
                .expect("binary-write recorder lock is available")
                .is_empty()
        );
        assert!(
            denied_discards
                .lock()
                .expect("binary-discard recorder lock is available")
                .is_empty()
        );

        let malformed_writer = RecordingFileBinaryWrite::accepting();
        let malformed_discards = Arc::clone(&malformed_writer.discarded);
        let malformed = JsonValue::parse(
            &file_binary_write_host(vec![Capability::FileWriteBinary], malformed_writer)
                .handle_json(&request_v1_22(
                    "file.write_binary",
                    &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AB"}}"#),
                )),
        )
        .expect("malformed response is JSON");
        assert_eq!(
            field(field(&malformed, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
        assert_eq!(
            malformed_discards
                .lock()
                .expect("binary-discard recorder lock is available")
                .as_slice(),
            [SaveReference::new(reference).expect("reference is valid")]
        );

        let oversized_writer = RecordingFileBinaryWrite::accepting();
        let oversized_discards = Arc::clone(&oversized_writer.discarded);
        let oversized = JsonValue::parse(
            &file_binary_write_host(vec![Capability::FileWriteBinary], oversized_writer)
                .handle_json(&request_v1_22(
                    "file.write_binary",
                    &format!(
                        r#"{{"saveReference":"{reference}","bytesBase64Url":"{}"}}"#,
                        "AAAA".repeat((anodrel_file_access::MAX_FILE_BINARY_WRITE_BYTES / 3) + 1)
                    ),
                )),
        )
        .expect("oversized response is JSON");
        assert_eq!(
            field(field(&oversized, "error"), "code").as_string(),
            Some("file.binary_too_large")
        );
        assert_eq!(
            oversized_discards
                .lock()
                .expect("binary-discard recorder lock is available")
                .as_slice(),
            [SaveReference::new(reference).expect("reference is valid")]
        );

        let unavailable = JsonValue::parse(
            &file_binary_write_host(
                vec![Capability::FileWriteBinary],
                RecordingFileBinaryWrite::unavailable(),
            )
            .handle_json(&request_v1_22(
                "file.write_binary",
                &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AA"}}"#),
            )),
        )
        .expect("unavailable response is JSON");
        assert_eq!(
            field(field(&unavailable, "error"), "code").as_string(),
            Some("file.unavailable")
        );

        let unsupported = JsonValue::parse(&accepted_host.handle_json(&request_v1_21(
            "file.write_binary",
            &format!(r#"{{"saveReference":"{reference}","bytesBase64Url":"AA"}}"#),
        )))
        .expect("unsupported response is JSON");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn selection_dialog_requires_the_open_grant_and_returns_an_opaque_reference() {
        let accepted_host = file_access_host(
            vec![Capability::DialogOpenFile],
            CapturingFileDialog,
            FixedFileText(Err(FileTextServiceError::Unavailable)),
        );
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
            "dialog.open_file.v2",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("selection response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "selectionReference").as_string(),
            Some("AbCdEfGhIjKlMnOpQrStUv")
        );

        let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_9(
            "dialog.open_file.v2",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("denied response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
            "dialog.open_file.v2",
            r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
        )))
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn selected_file_text_is_separately_granted_bounded_and_safe() {
        let reference = "AbCdEfGhIjKlMnOpQrStUv";
        let accepted_host = file_access_host(
            vec![Capability::FileReadText],
            CapturingFileDialog,
            FixedFileText(Ok("selected text".to_owned())),
        );
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
            "file.read_text",
            &format!(r#"{{"selectionReference":"{reference}"}}"#),
        )))
        .expect("text response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "text").as_string(),
            Some("selected text")
        );

        let denied = JsonValue::parse(&host(vec![Capability::DialogOpenFile]).handle_json(
            &request_v1_9(
                "file.read_text",
                &format!(r#"{{"selectionReference":"{reference}"}}"#),
            ),
        ))
        .expect("denied response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
            "file.read_text",
            r#"{"selectionReference":"path.txt"}"#,
        )))
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        for (service_error, expected) in [
            (FileTextServiceError::Unavailable, "file.unavailable"),
            (FileTextServiceError::InvalidText, "file.text_invalid"),
            (FileTextServiceError::TooLarge, "file.text_too_large"),
        ] {
            let failing_host = file_access_host(
                vec![Capability::FileReadText],
                CapturingFileDialog,
                FixedFileText(Err(service_error)),
            );
            let response = JsonValue::parse(&failing_host.handle_json(&request_v1_9(
                "file.read_text",
                &format!(r#"{{"selectionReference":"{reference}"}}"#),
            )))
            .expect("failure response is JSON");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(expected)
            );
        }

        let oversized_host = file_access_host(
            vec![Capability::FileReadText],
            CapturingFileDialog,
            FixedFileText(Ok("x".repeat(MAX_FILE_TEXT_RESPONSE_BYTES + 1))),
        );
        let oversized = JsonValue::parse(&oversized_host.handle_json(&request_v1_9(
            "file.read_text",
            &format!(r#"{{"selectionReference":"{reference}"}}"#),
        )))
        .expect("oversized response is JSON");
        assert_eq!(
            field(field(&oversized, "error"), "code").as_string(),
            Some("file.text_too_large")
        );
    }

    #[test]
    fn storage_operations_are_exact_bounded_and_independently_granted() {
        let storage_host = storage_host(
            vec![
                Capability::StorageStateRead,
                Capability::StorageStateReplace,
                Capability::StorageStateClear,
            ],
            MemoryStorage::with_state(StorageRead::Absent),
        );
        let replaced = JsonValue::parse(&storage_host.handle_json(&request_v1_10(
            "storage.state.replace",
            r#"{"snapshot":"saved"}"#,
        )))
        .expect("replace response is JSON");
        assert_eq!(
            field(field(&replaced, "result"), "status").as_string(),
            Some("replaced")
        );

        let read =
            JsonValue::parse(&storage_host.handle_json(&request_v1_10("storage.state.read", "{}")))
                .expect("read response is JSON");
        assert_eq!(
            field(field(&read, "result"), "snapshot").as_string(),
            Some("saved")
        );

        let cleared = JsonValue::parse(
            &storage_host.handle_json(&request_v1_10("storage.state.clear", "{}")),
        )
        .expect("clear response is JSON");
        assert_eq!(
            field(field(&cleared, "result"), "status").as_string(),
            Some("cleared")
        );

        let invalid = JsonValue::parse(
            &storage_host.handle_json(&request_v1_10("storage.state.read", r#"{"extra":true}"#)),
        )
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let no_grant =
            JsonValue::parse(&host(vec![]).handle_json(&request_v1_10("storage.state.read", "{}")))
                .expect("denied response is JSON");
        assert_eq!(
            field(field(&no_grant, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let oversized = format!(
            r#"{{"snapshot":"{}"}}"#,
            "x".repeat(MAX_STORAGE_SNAPSHOT_REQUEST_BYTES + 1)
        );
        let rejected = JsonValue::parse(
            &storage_host.handle_json(&request_v1_10("storage.state.replace", &oversized)),
        )
        .expect("oversized response is JSON");
        assert_eq!(
            field(field(&rejected, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn credential_operations_are_exact_and_independently_granted() {
        let service_host = credential_host(
            vec![
                Capability::CredentialRead,
                Capability::CredentialWrite,
                Capability::CredentialDelete,
            ],
            MemoryCredentials::default(),
        );
        let absent = JsonValue::parse(&service_host.handle_json(&request_v1_12(
            "credential.read",
            r#"{"name":"refresh-token"}"#,
        )))
        .expect("absent response is JSON");
        assert_eq!(
            field(field(&absent, "result"), "status").as_string(),
            Some("not_found")
        );

        let written = JsonValue::parse(&service_host.handle_json(&request_v1_12(
            "credential.write",
            r#"{"name":"refresh-token","secret":"00aaff"}"#,
        )))
        .expect("write response is JSON");
        assert_eq!(
            field(field(&written, "result"), "status").as_string(),
            Some("written")
        );

        let found = JsonValue::parse(&service_host.handle_json(&request_v1_12(
            "credential.read",
            r#"{"name":"refresh-token"}"#,
        )))
        .expect("read response is JSON");
        assert_eq!(
            field(field(&found, "result"), "secret").as_string(),
            Some("00aaff")
        );

        let deleted = JsonValue::parse(&service_host.handle_json(&request_v1_12(
            "credential.delete",
            r#"{"name":"refresh-token"}"#,
        )))
        .expect("delete response is JSON");
        assert_eq!(
            field(field(&deleted, "result"), "status").as_string(),
            Some("deleted")
        );

        let invalid = JsonValue::parse(&service_host.handle_json(&request_v1_12(
            "credential.write",
            r#"{"name":"refresh-token","secret":"ABCDEF"}"#,
        )))
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let denied = JsonValue::parse(
            &credential_host(
                vec![Capability::CredentialRead],
                MemoryCredentials::default(),
            )
            .handle_json(&request_v1_12(
                "credential.write",
                r#"{"name":"refresh-token","secret":"00aaff"}"#,
            )),
        )
        .expect("denied response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let unsupported = JsonValue::parse(&service_host.handle_json(&request_v1_10(
            "credential.read",
            r#"{"name":"refresh-token"}"#,
        )))
        .expect("unsupported response is JSON");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn service_bundle_exposes_only_the_explicitly_attached_services() {
        let host = CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::CredentialWrite, Capability::StorageStateRead],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable()
                .with_credentials(MemoryCredentials::default())
                .with_storage(MemoryStorage::with_state(StorageRead::Absent)),
        );

        let credential = JsonValue::parse(&host.handle_json(&request_v1_12(
            "credential.write",
            r#"{"name":"refresh-token","secret":"00aaff"}"#,
        )))
        .expect("credential response is JSON");
        assert_eq!(
            field(field(&credential, "result"), "status").as_string(),
            Some("written")
        );

        let storage =
            JsonValue::parse(&host.handle_json(&request_v1_10("storage.state.read", "{}")))
                .expect("storage response is JSON");
        assert_eq!(
            field(field(&storage, "result"), "status").as_string(),
            Some("absent")
        );

        let unavailable =
            JsonValue::parse(&host.handle_json(&request_v1_5("clipboard.read", "{}")))
                .expect("clipboard response is JSON");
        assert_eq!(
            field(field(&unavailable, "error"), "code").as_string(),
            Some("capability.denied")
        );
    }

    #[test]
    fn rejects_duplicate_host_capability_grants() {
        assert!(
            HostPolicy::new(
                "test.application",
                vec![Capability::DiagnosticsRead, Capability::DiagnosticsRead],
                "test-host",
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_unsupported_versions_and_oversized_messages() {
        let unsupported = request("platform.ping", r#"{"sentAt":"now"}"#).replacen(
            "\"major\":1",
            "\"major\":2",
            1,
        );
        let response =
            JsonValue::parse(&host(vec![]).handle_json(&unsupported)).expect("valid JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("protocol.version_unsupported")
        );

        let response =
            JsonValue::parse(&host(vec![]).handle_json(&"x".repeat(MAX_REQUEST_BYTES + 1)))
                .expect("valid JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.invalid")
        );
    }

    #[test]
    fn converts_known_epoch_days_without_a_time_library() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_300), (2025, 7, 31));
    }
}
