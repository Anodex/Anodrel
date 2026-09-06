//! Closed capability sets for each host-selected development template.

use anodrel_protocol::Capability;

pub(crate) const UI_GRANTS: [Capability; 3] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::SessionClose,
];
pub(crate) const FORM_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::UiFieldsRead,
    Capability::SessionClose,
];
pub(crate) const MENU_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::MenuWrite,
    Capability::SessionClose,
];
pub(crate) const CONTEXT_MENU_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::ContextMenuWrite,
    Capability::SessionClose,
];
pub(crate) const TRAY_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::TrayWrite,
    Capability::SessionClose,
];
pub(crate) const NOTIFICATION_GRANTS: [Capability; 3] = [
    Capability::UiDocumentWrite,
    Capability::NotificationShow,
    Capability::SessionClose,
];
pub(crate) const FILE_WRITE_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::DialogSaveFile,
    Capability::FileWriteText,
    Capability::SessionClose,
];
pub(crate) const FILE_BINARY_WRITE_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::DialogSaveFile,
    Capability::FileWriteBinary,
    Capability::SessionClose,
];
pub(crate) const MULTI_WINDOW_GRANTS: [Capability; 5] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::WindowOpen,
    Capability::WindowClose,
    Capability::SessionClose,
];
pub(crate) const WINDOW_CONTROLS_GRANTS: [Capability; 8] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::WindowTitle,
    Capability::WindowState,
    Capability::WindowFocus,
    Capability::WindowFullscreen,
    Capability::WindowSize,
    Capability::SessionClose,
];
