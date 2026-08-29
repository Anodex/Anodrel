//! Explicit native-service bundle and safe unavailable defaults.
//!
//! Host composition uses this module to name every platform service before an
//! authenticated session begins. Nothing can acquire ambient authority later.

use super::*;

/// Explicit native services owned by one authenticated host session.
///
/// A native composition boundary builds this value from host-validated
/// identity, policy, and native resources before a transport session begins.
/// Protocol messages cannot replace, inspect, or add services after this value
/// has been consumed by [`CoreHost`]. Services are unavailable by default so a
/// newly declared capability never gains ambient operating-system authority.
#[derive(Debug)]
pub struct HostServices {
    pub(super) clipboard: Box<dyn ClipboardService>,
    pub(super) external_links: Box<dyn ExternalLinkService>,
    pub(super) network: Box<dyn NetworkTextService>,
    pub(super) notifications: Box<dyn NotificationService>,
    pub(super) window_title: Box<dyn WindowTitleService>,
    pub(super) window_state: Box<dyn WindowStateService>,
    pub(super) window_state_read: Box<dyn WindowStateReadService>,
    pub(super) window_focus: Box<dyn WindowFocusService>,
    pub(super) window_fullscreen: Box<dyn WindowFullscreenService>,
    pub(super) window_size: Box<dyn WindowSizeService>,
    pub(super) menu: Box<dyn MenuService>,
    pub(super) ui_fields: Box<dyn UiFieldReader>,
    pub(super) file_dialogs: Box<dyn FileDialogService>,
    pub(super) folder_selections: Box<dyn FolderSelectionService>,
    pub(super) folder_entries: Box<dyn FolderEntryService>,
    pub(super) file_selections: Box<dyn FileSelectionService>,
    pub(super) file_text: Box<dyn FileTextService>,
    pub(super) file_save_selections: Box<dyn SaveSelectionService>,
    pub(super) file_text_write: Box<dyn FileTextWriteService>,
    pub(super) file_binary_write: Box<dyn FileBinaryWriteService>,
    pub(super) storage: Box<dyn StorageService>,
    pub(super) diagnostics: Box<dyn DiagnosticsService>,
    pub(super) credentials: Box<dyn CredentialService>,
}

#[derive(Debug)]
pub(super) struct UnavailableClipboard;

impl ClipboardService for UnavailableClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }

    fn write_text(&self, _text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableExternalLinks;

impl ExternalLinkService for UnavailableExternalLinks {
    fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        Err(ExternalLinkOpenError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableNetwork;

impl NetworkTextService for UnavailableNetwork {
    fn fetch_text(
        &self,
        _url: &NetworkUrl,
    ) -> Result<anodrel_network::NetworkTextResponse, NetworkTextServiceError> {
        Err(NetworkTextServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableNotifications;

impl NotificationService for UnavailableNotifications {
    fn show(&self, _notification: &Notification) -> Result<(), NotificationServiceError> {
        Err(NotificationServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableUiFields;

impl UiFieldReader for UnavailableUiFields {
    fn read(&self) -> Result<UiFieldSnapshot, UiFieldReadError> {
        Err(UiFieldReadError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableWindowTitle;

impl WindowTitleService for UnavailableWindowTitle {
    fn set_title(&self, _proposal: &WindowTitleProposal) -> Result<(), WindowTitleServiceError> {
        Err(WindowTitleServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableWindowState;

impl WindowStateService for UnavailableWindowState {
    fn set_state(&self, _state: WindowState) -> Result<(), WindowStateServiceError> {
        Err(WindowStateServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableWindowStateRead;

impl WindowStateReadService for UnavailableWindowStateRead {
    fn read_state(&self) -> Result<WindowState, WindowStateReadServiceError> {
        Err(WindowStateReadServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableWindowFocus;

impl WindowFocusService for UnavailableWindowFocus {
    fn request_focus(&self) -> Result<(), WindowFocusServiceError> {
        Err(WindowFocusServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableWindowFullscreen;

impl WindowFullscreenService for UnavailableWindowFullscreen {
    fn set_fullscreen(
        &self,
        _mode: WindowFullscreenMode,
    ) -> Result<(), WindowFullscreenServiceError> {
        Err(WindowFullscreenServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableWindowSize;

impl WindowSizeService for UnavailableWindowSize {
    fn set_size(&self, _size: WindowSize) -> Result<(), WindowSizeServiceError> {
        Err(WindowSizeServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableFileDialogs;

impl FileDialogService for UnavailableFileDialogs {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableStorage;

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
pub(super) struct UnavailableDiagnostics;

impl DiagnosticsService for UnavailableDiagnostics {
    fn entries(&self) -> Result<Vec<anodrel_diagnostics::Entry>, DiagnosticsServiceError> {
        Err(DiagnosticsServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(super) struct UnavailableCredentials;

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
            window_state_read: Box::new(UnavailableWindowStateRead),
            window_focus: Box::new(UnavailableWindowFocus),
            window_fullscreen: Box::new(UnavailableWindowFullscreen),
            window_size: Box::new(UnavailableWindowSize),
            menu: Box::new(UnavailableMenuService),
            ui_fields: Box::new(UnavailableUiFields),
            file_dialogs: Box::new(UnavailableFileDialogs),
            folder_selections: Box::new(UnavailableFolderSelectionService),
            folder_entries: Box::new(UnavailableFolderEntryService),
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

    /// Replaces the session's pull-only window-state observation service.
    ///
    /// The service samples only the requesting session's own window from the
    /// host UI thread. It accepts no target and returns no native detail or
    /// change stream; see `docs/WINDOW_STATE_OBSERVATION.md`.
    #[must_use]
    pub fn with_window_state_read(
        mut self,
        service: impl WindowStateReadService + 'static,
    ) -> Self {
        self.window_state_read = Box::new(service);
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

    /// Replaces the session's retained folder-selection service.
    #[must_use]
    pub fn with_folder_selections(
        mut self,
        service: impl FolderSelectionService + 'static,
    ) -> Self {
        self.folder_selections = Box::new(service);
        self
    }

    /// Replaces the session's bounded selected-folder entry service.
    #[must_use]
    pub fn with_folder_entries(mut self, service: impl FolderEntryService + 'static) -> Self {
        self.folder_entries = Box::new(service);
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
