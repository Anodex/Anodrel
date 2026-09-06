//! Exact closed-grant checks for host-selected development templates.

use anodrel_protocol::Capability;

use super::{
    grants::{
        CONTEXT_MENU_GRANTS, FILE_BINARY_WRITE_GRANTS, FILE_WRITE_GRANTS, FORM_GRANTS, MENU_GRANTS,
        MULTI_WINDOW_GRANTS, NOTIFICATION_GRANTS, TRAY_GRANTS, UI_GRANTS, WINDOW_CONTROLS_GRANTS,
    },
    templates::DevelopmentUiSessionConfig,
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
    let file_write = DevelopmentUiSessionConfig::with_file_write(
        "anodrel.test-file-write",
        "test-file-write-session",
        "Anodrel File Write Test",
        "completed file write",
    );
    let file_binary_write = DevelopmentUiSessionConfig::with_file_binary_write(
        "anodrel.test-file-binary-write",
        "test-file-binary-write-session",
        "Anodrel File Binary Write Test",
        "completed file binary write",
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
    assert_eq!(file_write.grants(), FILE_WRITE_GRANTS);
    assert!(file_write.supports_file_write());
    assert!(!file_write.supports_notification());
    assert!(!file_write.supports_fields());
    assert!(!file_write.supports_menu());
    assert_eq!(file_binary_write.grants(), FILE_BINARY_WRITE_GRANTS);
    assert!(file_binary_write.supports_file_binary_write());
    assert!(!file_binary_write.supports_file_write());
    assert!(!file_binary_write.supports_notification());
    assert!(!file_binary_write.supports_menu());
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
