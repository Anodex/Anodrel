//! Host-owned resources for one development UI session.
//!
//! The Node and compiled-native diagnostics use the same window implementation
//! but choose different child executables. Keeping the resource bundle here
//! prevents either diagnostic from constructing a subtly different session or
//! growing a positional constructor at its call site.

use anodrel_core::SessionCloseSignal;
use anodrel_file_dialog::FileDialogMailbox;
use anodrel_ui_session::{UiDocumentMailbox, UiFieldMailbox, UiInputMailbox};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_folder_access::WindowsFolderEntryService;

/// The complete set of host-only bridges consumed by one development session.
///
/// A diagnostic grants only the capabilities it needs; ungranted bridges here
/// remain host-owned and unreachable through the authenticated protocol.
pub(crate) struct DevelopmentSessionUi {
    pub(crate) document: UiDocumentMailbox,
    pub(crate) input: UiInputMailbox,
    pub(crate) close: SessionCloseSignal,
    pub(crate) file_dialog: FileDialogMailbox,
    pub(crate) file_text: WindowsFileTextService,
    pub(crate) folder_entries: WindowsFolderEntryService,
    pub(crate) notifications: anodrel_notifications::NotificationMailbox,
    pub(crate) menu: anodrel_menu::MenuMailbox,
    pub(crate) context_menu: anodrel_menu::ContextMenuMailbox,
    pub(crate) tray: anodrel_menu::TrayMailbox,
    pub(crate) window_title: anodrel_window::WindowTitleMailbox,
    pub(crate) window_state: anodrel_window::WindowStateMailbox,
    pub(crate) window_state_read: anodrel_window::WindowStateReadMailbox,
    pub(crate) window_state_changes: anodrel_window::WindowStateChangesMailbox,
    pub(crate) window_focus: anodrel_window::WindowFocusMailbox,
    pub(crate) window_fullscreen: anodrel_window::WindowFullscreenMailbox,
    pub(crate) window_size: anodrel_window::WindowSizeMailbox,
    pub(crate) fields: UiFieldMailbox,
}

impl DevelopmentSessionUi {
    /// Creates a fresh, non-shared resource bundle for exactly one session.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            document: UiDocumentMailbox::new(),
            input: UiInputMailbox::new(),
            close: SessionCloseSignal::default(),
            file_dialog: FileDialogMailbox::new(),
            file_text: WindowsFileTextService::new(),
            folder_entries: WindowsFolderEntryService::new(),
            notifications: anodrel_notifications::NotificationMailbox::new(),
            menu: anodrel_menu::MenuMailbox::new(),
            context_menu: anodrel_menu::ContextMenuMailbox::new(),
            tray: anodrel_menu::TrayMailbox::new(),
            window_title: anodrel_window::WindowTitleMailbox::new(),
            window_state: anodrel_window::WindowStateMailbox::new(),
            window_state_read: anodrel_window::WindowStateReadMailbox::new(),
            window_state_changes: anodrel_window::WindowStateChangesMailbox::new(),
            window_focus: anodrel_window::WindowFocusMailbox::new(),
            window_fullscreen: anodrel_window::WindowFullscreenMailbox::new(),
            window_size: anodrel_window::WindowSizeMailbox::new(),
            fields: UiFieldMailbox::new(),
        }
    }
}
