//! Shared Shell32 notification-area creation and tray mailbox servicing.
//!
//! The one host-selected icon is deliberately created only on demand and is
//! retained by the session view, whether notifications or tray interaction
//! reach it first.

use super::super::*;

/// Shows one pending notification for a session window, if it has one.
///
/// The Shell32 call runs outside the window registry's lock, so a slow shell
/// cannot block every other window's message handling. The entry is created on
/// first use and then shared with any later tray model.
pub(in crate::win32) fn service_notification(window: Hwnd) {
    let Ok(Some((request, entry))) = registry::take_notification_request(window) else {
        return;
    };

    let (entry, created) = match entry {
        Some(entry) => (Some(entry), None),
        None => match create_notification_area(window) {
            Ok(entry) => {
                let entry = std::sync::Arc::new(entry);
                (Some(std::sync::Arc::clone(&entry)), Some(entry))
            }
            Err(()) => (None, None),
        },
    };
    let shown = entry.is_some_and(|entry| {
        let notifications =
            anodrel_windows_notifications::WindowsNotifications::from_notification_area(entry);
        anodrel_notifications::NotificationService::show(&notifications, request.notification())
            .is_ok()
    });
    let _ = registry::complete_notification_request(window, request.id(), shown, created);
}

/// Applies one pending semantic tray replacement on the owning UI thread.
///
/// The entry and temporary User32 probe are built outside the registry lock. A
/// failed replacement preserves an existing entry and menu; a fresh entry is
/// dropped unless the full replacement commits.
pub(in crate::win32) fn service_tray(window: Hwnd) {
    let Ok(Some((request, entry))) = registry::take_tray_request(window) else {
        return;
    };

    let (entry, created) = match entry {
        Some(entry) => (Some(entry), None),
        None => match create_notification_area(window) {
            Ok(entry) => {
                let entry = std::sync::Arc::new(entry);
                (Some(std::sync::Arc::clone(&entry)), Some(entry))
            }
            Err(()) => (None, None),
        },
    };
    let tray = entry
        .as_ref()
        .filter(|entry| {
            entry
                .set_callback_message(WM_ANODREL_NOTIFICATION_AREA)
                .is_ok()
        })
        .and_then(|_| tray::TrayMenu::build(&request));
    let applied = match tray {
        Some(tray) => registry::replace_tray(window, tray, created)
            .ok()
            .flatten()
            .unwrap_or(false),
        None => false,
    };
    let _ = registry::complete_tray_request(window, request.id(), applied);
}

/// Creates the one shared host-selected notification-area entry for a view.
///
/// Its icon and tooltip are fixed host values; no application protocol payload
/// reaches Shell32.
fn create_notification_area(
    window: Hwnd,
) -> Result<anodrel_windows_notification_area::NotificationArea, ()> {
    anodrel_windows_notification_area::NotificationArea::create(
        window,
        ICONS.get_or_init(appicon::create).0.unwrap_or(0),
        anodrel_windows_notifications::ENTRY_TIP,
    )
    .map_err(|_| ())
}
