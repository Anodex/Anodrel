//! Host-owned UI resources for one registered Windows session.

use anodrel_core::SessionCloseSignal;
use anodrel_file_dialog::FileDialogMailbox;
use anodrel_menu::{ContextMenuMailbox, MenuMailbox, TrayMailbox};
use anodrel_notifications::NotificationMailbox;
use anodrel_ui_session::{
    UiDocumentMailbox, UiFieldMailbox, UiInputMailbox, UiWindowGroup, UiWindowId,
};
use anodrel_window::{
    WindowFocusMailbox, WindowFullscreenMailbox, WindowSizeMailbox, WindowStateChangesMailbox,
    WindowStateMailbox, WindowStateReadMailbox, WindowTitleMailbox, WindowTitleProposal,
};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_folder_access::WindowsFolderEntryService;

/// The host-owned native UI resources for one registered application session.
///
/// This group has no native handle, process, title, launch operation, or
/// application-selected data. Host code must pass it only to the native window
/// that belongs to the same session returned by [`crate::RegisteredUiSession`].
#[derive(Clone, Debug)]
pub struct RegisteredSessionUi {
    window_group: UiWindowGroup<WindowTitleProposal>,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    folder_entries: WindowsFolderEntryService,
    notification_mailbox: NotificationMailbox,
    menu_mailbox: MenuMailbox,
    context_menu_mailbox: ContextMenuMailbox,
    tray_mailbox: TrayMailbox,
    window_title_mailbox: WindowTitleMailbox,
    window_state_mailbox: WindowStateMailbox,
    window_state_read_mailbox: WindowStateReadMailbox,
    window_state_changes_mailbox: WindowStateChangesMailbox,
    window_focus_mailbox: WindowFocusMailbox,
    window_fullscreen_mailbox: WindowFullscreenMailbox,
    window_size_mailbox: WindowSizeMailbox,
    field_mailbox: UiFieldMailbox,
    /// The display name the host appends to any title this session proposes.
    ///
    /// Taken from the identity that matched the machine-validated installed
    /// record, so it is not something the application can influence at run
    /// time. Carrying it here is what lets the UI thread compose a caption an
    /// application cannot forge. See `docs/WINDOW_TITLE.md`.
    display_name: String,
    /// The machine-validated application identity eligible for the one native
    /// product-update action, when signed policy selected an update catalogue.
    ///
    /// This private host value is never placed in a protocol response or an
    /// application menu. The Windows window uses it only after a local click
    /// on its fixed system-menu command.
    update_application_id: Option<String>,
}

impl RegisteredSessionUi {
    #[cfg(test)]
    pub(crate) fn new(display_name: impl Into<String>) -> Self {
        Self::with_update_action(display_name, None)
    }

    pub(crate) fn with_update_action(
        display_name: impl Into<String>,
        update_application_id: Option<&str>,
    ) -> Self {
        let document_mailbox = UiDocumentMailbox::new();
        let input_mailbox = UiInputMailbox::new();
        Self {
            window_group: UiWindowGroup::with_primary_resources(document_mailbox, input_mailbox),
            close_signal: SessionCloseSignal::default(),
            file_dialog_mailbox: FileDialogMailbox::new(),
            file_text: WindowsFileTextService::new(),
            folder_entries: WindowsFolderEntryService::new(),
            notification_mailbox: NotificationMailbox::new(),
            menu_mailbox: MenuMailbox::new(),
            context_menu_mailbox: ContextMenuMailbox::new(),
            tray_mailbox: TrayMailbox::new(),
            window_title_mailbox: WindowTitleMailbox::new(),
            window_state_mailbox: WindowStateMailbox::new(),
            window_state_read_mailbox: WindowStateReadMailbox::new(),
            window_state_changes_mailbox: WindowStateChangesMailbox::new(),
            window_focus_mailbox: WindowFocusMailbox::new(),
            window_fullscreen_mailbox: WindowFullscreenMailbox::new(),
            window_size_mailbox: WindowSizeMailbox::new(),
            field_mailbox: UiFieldMailbox::new(),
            display_name: display_name.into(),
            update_application_id: update_application_id.map(str::to_owned),
        }
    }

    /// Returns this session's document-delivery mailbox.
    #[must_use]
    pub fn document_mailbox(&self) -> UiDocumentMailbox {
        self.primary_resources().document_mailbox()
    }

    /// Returns this session's bounded semantic-input mailbox.
    #[must_use]
    pub fn input_mailbox(&self) -> UiInputMailbox {
        self.primary_resources().input_mailbox()
    }

    /// Returns this registered session's host-owned portable view group.
    ///
    /// This value has no native handle or application-selected state. The
    /// Windows host uses it only for the same registered session's UI-thread
    /// lifecycle and authenticated transport composition.
    #[must_use]
    pub fn window_group(&self) -> UiWindowGroup<WindowTitleProposal> {
        self.window_group.clone()
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

    /// Returns this session's retained selected-folder entry service.
    #[must_use]
    pub fn folder_entry_service(&self) -> WindowsFolderEntryService {
        self.folder_entries.clone()
    }

    /// Returns this session's one-request UI-thread notification mailbox.
    #[must_use]
    pub fn notification_mailbox(&self) -> NotificationMailbox {
        self.notification_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread menu mailbox.
    ///
    /// It carries only a complete validated semantic model and host-owned
    /// revision. The UI thread that owns this session's window supplies every
    /// native menu object and private command identifier.
    #[must_use]
    pub fn menu_mailbox(&self) -> MenuMailbox {
        self.menu_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread context-menu mailbox.
    ///
    /// The complete semantic model stays distinct from the session menu bar.
    /// The Windows view owns local triggers, popup placement, native command
    /// IDs, and action routing; see `docs/CONTEXT_MENUS.md`.
    #[must_use]
    pub fn context_menu_mailbox(&self) -> ContextMenuMailbox {
        self.context_menu_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread notification-area tray mailbox.
    ///
    /// The native icon, callback, popup placement, and private command mapping
    /// stay with the Windows host; this carries only a complete semantic model.
    #[must_use]
    pub fn tray_mailbox(&self) -> TrayMailbox {
        self.tray_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread window-title mailbox.
    #[must_use]
    pub fn window_title_mailbox(&self) -> WindowTitleMailbox {
        self.window_title_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread window-state mailbox.
    ///
    /// It transfers only the closed minimise, maximise, or restore value. The
    /// native window remains resolved by the host, never by the application.
    #[must_use]
    pub fn window_state_mailbox(&self) -> WindowStateMailbox {
        self.window_state_mailbox.clone()
    }

    /// Returns this session's pull-only UI-thread state-observation mailbox.
    ///
    /// It transfers only one immediate closed presentation snapshot. The
    /// native window remains resolved by the host, and no target, geometry,
    /// focus, timestamp, or event reaches the application.
    #[must_use]
    pub fn window_state_read_mailbox(&self) -> WindowStateReadMailbox {
        self.window_state_read_mailbox.clone()
    }

    /// Returns this session's coalesced pull-only state-change mailbox.
    ///
    /// It retains one latest closed transition for the host-resolved session
    /// window. It has no target, native handle, history, timestamp, wait,
    /// callback, or subscription.
    #[must_use]
    pub fn window_state_changes_mailbox(&self) -> WindowStateChangesMailbox {
        self.window_state_changes_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread window-focus mailbox.
    ///
    /// It carries no target, native handle, retry policy, or focus readback.
    /// The owning UI thread resolves the one session window; see
    /// `docs/WINDOW_FOCUS.md`.
    #[must_use]
    pub fn window_focus_mailbox(&self) -> WindowFocusMailbox {
        self.window_focus_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread fullscreen mailbox.
    ///
    /// It carries only the two reversible presentation modes. The owning UI
    /// thread retains every native style and placement fact and resolves the
    /// one session window; see `docs/WINDOW_FULLSCREEN.md`.
    #[must_use]
    pub fn window_fullscreen_mailbox(&self) -> WindowFullscreenMailbox {
        self.window_fullscreen_mailbox.clone()
    }

    /// Returns this session's one-request logical client-size mailbox.
    ///
    /// It carries only bounded dimensions. The owning UI thread resolves the
    /// session window and derives its own outer frame; see `docs/WINDOW_SIZE.md`.
    #[must_use]
    pub fn window_size_mailbox(&self) -> WindowSizeMailbox {
        self.window_size_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread field-read mailbox.
    #[must_use]
    pub fn field_mailbox(&self) -> UiFieldMailbox {
        self.field_mailbox.clone()
    }

    /// Returns the validated display name the host appends to a proposed title.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the host-held identity for the fixed native update action.
    ///
    /// `None` means the signed selected record has no update catalogue, so the
    /// product window must not add an update action at all. This identity is
    /// for internal Windows host composition only and never reaches the pipe.
    #[must_use]
    pub fn update_application_id(&self) -> Option<&str> {
        self.update_application_id.as_deref()
    }

    fn primary_resources(&self) -> anodrel_ui_session::UiWindowResources {
        self.window_group
            .resources(&UiWindowId::primary())
            .expect("a session-owned group always retains its primary view")
    }
}
