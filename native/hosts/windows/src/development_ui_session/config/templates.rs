//! Closed selection and capability mapping for development UI templates.

use anodrel_protocol::Capability;

use super::grants::*;

#[derive(Clone, Copy)]
enum DevelopmentUiSessionKind {
    Document,
    Form,
    Menu,
    ContextMenu,
    Tray,
    Notification,
    FileWrite,
    FileBinaryWrite,
    MultiWindow,
    WindowControls,
}

/// Fixed host facts for one explicitly selected development child route.
#[derive(Clone, Copy)]
pub(crate) struct DevelopmentUiSessionConfig {
    pub(crate) application_id: &'static str,
    pub(crate) session_id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) completion_message: &'static str,
    kind: DevelopmentUiSessionKind,
}

impl DevelopmentUiSessionConfig {
    /// Creates a configuration whose only session permissions are UI write,
    /// semantic-action read, and self-close.
    pub(crate) const fn new(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::Document,
        )
    }

    /// Creates a configuration for the explicit whole-surface field-read
    /// route. The other templates do not acquire this inward-facing authority.
    pub(crate) const fn with_form(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::Form,
        )
    }

    /// Creates a configuration whose only additional session permission is a
    /// complete bounded native-menu replacement.
    pub(crate) const fn with_menu(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::Menu,
        )
    }

    /// Creates a configuration whose only additional permission is a complete
    /// host-owned context-menu replacement.
    pub(crate) const fn with_context_menu(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::ContextMenu,
        )
    }

    /// Creates a configuration whose only additional permission is one
    /// complete host-owned notification-area tray replacement.
    pub(crate) const fn with_tray(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::Tray,
        )
    }

    /// Creates a configuration whose only additional permission is one
    /// bounded one-way native notification.
    pub(crate) const fn with_notification(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::Notification,
        )
    }

    /// Creates a configuration for one retained selected-output text write.
    ///
    /// The host retains the selected native object. This route does not grant
    /// a path-based filesystem operation, input events, or output readback.
    pub(crate) const fn with_file_write(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::FileWrite,
        )
    }

    /// Creates a configuration for one retained selected-output binary write.
    ///
    /// The host retains the selected native object. This route does not grant
    /// a path-based filesystem operation, input events, or output readback.
    pub(crate) const fn with_file_binary_write(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::FileBinaryWrite,
        )
    }

    /// Creates a configuration for the explicit bounded multi-window route.
    ///
    /// Window creation and secondary close are additional fixed grants on this
    /// distinct route. The normal and menu routes remain narrower.
    pub(crate) const fn with_multi_window(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::MultiWindow,
        )
    }

    /// Creates a configuration for the explicit targetless session-window
    /// controls route. No existing development template acquires these grants.
    pub(crate) const fn with_window_controls(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self::with_kind(
            application_id,
            session_id,
            display_name,
            completion_message,
            DevelopmentUiSessionKind::WindowControls,
        )
    }

    const fn with_kind(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
        kind: DevelopmentUiSessionKind,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind,
        }
    }

    pub(crate) const fn grants(self) -> &'static [Capability] {
        match self.kind {
            DevelopmentUiSessionKind::Document => &UI_GRANTS,
            DevelopmentUiSessionKind::Form => &FORM_GRANTS,
            DevelopmentUiSessionKind::Menu => &MENU_GRANTS,
            DevelopmentUiSessionKind::ContextMenu => &CONTEXT_MENU_GRANTS,
            DevelopmentUiSessionKind::Tray => &TRAY_GRANTS,
            DevelopmentUiSessionKind::Notification => &NOTIFICATION_GRANTS,
            DevelopmentUiSessionKind::FileWrite => &FILE_WRITE_GRANTS,
            DevelopmentUiSessionKind::FileBinaryWrite => &FILE_BINARY_WRITE_GRANTS,
            DevelopmentUiSessionKind::MultiWindow => &MULTI_WINDOW_GRANTS,
            DevelopmentUiSessionKind::WindowControls => &WINDOW_CONTROLS_GRANTS,
        }
    }

    pub(crate) const fn supports_menu(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Menu)
    }

    pub(crate) const fn supports_context_menu(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::ContextMenu)
    }

    pub(crate) const fn supports_tray(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Tray)
    }

    pub(crate) const fn supports_notification(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Notification)
    }

    pub(crate) const fn supports_file_write(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::FileWrite)
    }

    pub(crate) const fn supports_file_binary_write(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::FileBinaryWrite)
    }

    pub(crate) const fn supports_fields(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Form)
    }

    pub(crate) const fn supports_multi_window(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::MultiWindow)
    }

    pub(crate) const fn supports_window_controls(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::WindowControls)
    }
}
