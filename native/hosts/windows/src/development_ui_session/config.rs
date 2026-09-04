//! Closed development-template configuration and capability sets.

use anodrel_protocol::Capability;

pub(super) const UI_GRANTS: [Capability; 3] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::SessionClose,
];
pub(super) const FORM_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::UiFieldsRead,
    Capability::SessionClose,
];
pub(super) const MENU_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::MenuWrite,
    Capability::SessionClose,
];
pub(super) const CONTEXT_MENU_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::ContextMenuWrite,
    Capability::SessionClose,
];
pub(super) const TRAY_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::TrayWrite,
    Capability::SessionClose,
];
pub(super) const NOTIFICATION_GRANTS: [Capability; 3] = [
    Capability::UiDocumentWrite,
    Capability::NotificationShow,
    Capability::SessionClose,
];
pub(super) const MULTI_WINDOW_GRANTS: [Capability; 5] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::WindowOpen,
    Capability::WindowClose,
    Capability::SessionClose,
];
pub(super) const WINDOW_CONTROLS_GRANTS: [Capability; 8] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::WindowTitle,
    Capability::WindowState,
    Capability::WindowFocus,
    Capability::WindowFullscreen,
    Capability::WindowSize,
    Capability::SessionClose,
];

#[derive(Clone, Copy)]
enum DevelopmentUiSessionKind {
    Document,
    Form,
    Menu,
    ContextMenu,
    Tray,
    Notification,
    MultiWindow,
    WindowControls,
}

/// Fixed host facts for one explicitly selected development child route.
#[derive(Clone, Copy)]
pub(crate) struct DevelopmentUiSessionConfig {
    pub(super) application_id: &'static str,
    pub(super) session_id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) completion_message: &'static str,
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
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::Document,
        }
    }

    /// Creates a configuration for the explicit whole-surface field-read
    /// route. The other templates do not acquire this inward-facing authority.
    pub(crate) const fn with_form(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::Form,
        }
    }

    /// Creates a configuration whose only additional session permission is a
    /// complete bounded native-menu replacement.
    pub(crate) const fn with_menu(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::Menu,
        }
    }

    /// Creates a configuration whose only additional permission is a complete
    /// host-owned context-menu replacement.
    pub(crate) const fn with_context_menu(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::ContextMenu,
        }
    }

    /// Creates a configuration whose only additional permission is one
    /// complete host-owned notification-area tray replacement.
    pub(crate) const fn with_tray(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::Tray,
        }
    }

    /// Creates a configuration whose only additional permission is one
    /// bounded one-way native notification.
    pub(crate) const fn with_notification(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::Notification,
        }
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
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::MultiWindow,
        }
    }

    /// Creates a configuration for the explicit targetless session-window
    /// controls route. No existing development template acquires these grants.
    pub(crate) const fn with_window_controls(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::WindowControls,
        }
    }

    pub(super) const fn grants(self) -> &'static [Capability] {
        match self.kind {
            DevelopmentUiSessionKind::Document => &UI_GRANTS,
            DevelopmentUiSessionKind::Form => &FORM_GRANTS,
            DevelopmentUiSessionKind::Menu => &MENU_GRANTS,
            DevelopmentUiSessionKind::ContextMenu => &CONTEXT_MENU_GRANTS,
            DevelopmentUiSessionKind::Tray => &TRAY_GRANTS,
            DevelopmentUiSessionKind::Notification => &NOTIFICATION_GRANTS,
            DevelopmentUiSessionKind::MultiWindow => &MULTI_WINDOW_GRANTS,
            DevelopmentUiSessionKind::WindowControls => &WINDOW_CONTROLS_GRANTS,
        }
    }

    pub(super) const fn supports_menu(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Menu)
    }

    pub(super) const fn supports_context_menu(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::ContextMenu)
    }

    pub(super) const fn supports_tray(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Tray)
    }

    pub(super) const fn supports_notification(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Notification)
    }

    pub(super) const fn supports_fields(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Form)
    }

    pub(super) const fn supports_multi_window(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::MultiWindow)
    }

    pub(super) const fn supports_window_controls(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::WindowControls)
    }
}

#[cfg(test)]
mod tests {
    use anodrel_protocol::Capability;

    use super::{
        CONTEXT_MENU_GRANTS, DevelopmentUiSessionConfig, FORM_GRANTS, MENU_GRANTS,
        MULTI_WINDOW_GRANTS, NOTIFICATION_GRANTS, TRAY_GRANTS, UI_GRANTS, WINDOW_CONTROLS_GRANTS,
    };

    #[test]
    fn development_routes_use_only_their_exact_closed_grant_sets() {
        let document = DevelopmentUiSessionConfig::new(
            "anodrel.test",
            "test-session",
            "Anodrel Test",
            "completed",
        );
        let menu = DevelopmentUiSessionConfig::with_menu(
            "anodrel.test-menu",
            "test-menu-session",
            "Anodrel Menu Test",
            "completed menu",
        );
        let form = DevelopmentUiSessionConfig::with_form(
            "anodrel.test-form",
            "test-form-session",
            "Anodrel Form Test",
            "completed form",
        );
        let context_menu = DevelopmentUiSessionConfig::with_context_menu(
            "anodrel.test-context-menu",
            "test-context-menu-session",
            "Anodrel Context Menu Test",
            "completed context menu",
        );
        let tray = DevelopmentUiSessionConfig::with_tray(
            "anodrel.test-tray",
            "test-tray-session",
            "Anodrel Tray Test",
            "completed tray",
        );
        let notification = DevelopmentUiSessionConfig::with_notification(
            "anodrel.test-notification",
            "test-notification-session",
            "Anodrel Notification Test",
            "completed notification",
        );
        let multi_window = DevelopmentUiSessionConfig::with_multi_window(
            "anodrel.test-multi-window",
            "test-multi-window-session",
            "Anodrel Multi-Window Test",
            "completed multi-window",
        );
        let window_controls = DevelopmentUiSessionConfig::with_window_controls(
            "anodrel.test-window-controls",
            "test-window-controls-session",
            "Anodrel Window Controls Test",
            "completed window controls",
        );
        assert_eq!(document.application_id, "anodrel.test");
        assert_eq!(document.session_id, "test-session");
        assert_eq!(document.display_name, "Anodrel Test");
        assert_eq!(document.completion_message, "completed");
        assert_eq!(document.grants(), UI_GRANTS);
        assert!(!document.supports_menu());
        assert!(!document.supports_fields());
        assert_eq!(form.grants(), FORM_GRANTS);
        assert!(form.supports_fields());
        assert!(!form.supports_menu());
        assert_eq!(menu.grants(), MENU_GRANTS);
        assert!(menu.supports_menu());
        assert_eq!(context_menu.grants(), CONTEXT_MENU_GRANTS);
        assert!(context_menu.supports_context_menu());
        assert!(!context_menu.supports_menu());
        assert_eq!(tray.grants(), TRAY_GRANTS);
        assert!(tray.supports_tray());
        assert!(!tray.supports_menu());
        assert!(!tray.supports_context_menu());
        assert_eq!(notification.grants(), NOTIFICATION_GRANTS);
        assert!(notification.supports_notification());
        assert!(!notification.supports_tray());
        assert_eq!(multi_window.grants(), MULTI_WINDOW_GRANTS);
        assert!(!multi_window.supports_menu());
        assert!(multi_window.supports_multi_window());
        assert_eq!(window_controls.grants(), WINDOW_CONTROLS_GRANTS);
        assert!(window_controls.supports_window_controls());
        assert!(!window_controls.supports_menu());
        assert!(!window_controls.supports_fields());
        assert!(!window_controls.supports_multi_window());
        assert_eq!(
            UI_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            FORM_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::UiFieldsRead,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            MENU_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::MenuWrite,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            CONTEXT_MENU_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::ContextMenuWrite,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            NOTIFICATION_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::NotificationShow,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            MULTI_WINDOW_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::WindowOpen,
                Capability::WindowClose,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            WINDOW_CONTROLS_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::WindowTitle,
                Capability::WindowState,
                Capability::WindowFocus,
                Capability::WindowFullscreen,
                Capability::WindowSize,
                Capability::SessionClose,
            ]
        );
    }
}
