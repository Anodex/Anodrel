//! Per-session window resources and UI-thread mailbox bridges.

use super::*;

/// Polls one session view's explicitly supplied mailbox on the UI thread.
pub(crate) fn poll_ui_session(window: Hwnd) -> io::Result<Option<UiSessionPoll>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.poll())),
        _ => Ok(None),
    }
}

/// Registers a group-owned session view against the just-created native window.
///
/// The caller performs this before it shows the window. A legacy diagnostic
/// session returns `Some(false)`: it deliberately has no logical group mapping
/// to register.
pub(crate) fn register_ui_session_window(window: Hwnd) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.register_native_window(window))),
        _ => Ok(None),
    }
}

/// Takes one group-owned secondary-view creation handoff on the UI thread.
///
/// Any member may receive the timer message, but the portable group makes the
/// handoff take-once. The request retains neither a native handle nor an
/// application-visible mapping.
pub(crate) fn take_secondary_open_request(
    window: Hwnd,
) -> io::Result<Option<super::super::session_window_group::SessionWindowOpenRequest>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_secondary_open_request()),
        _ => Ok(None),
    }
}

/// Takes host-private native secondary windows requested for close by this
/// session group. The caller must invoke `DestroyWindow` only after this
/// registry lock has been released.
pub(crate) fn take_secondary_close_windows(window: Hwnd) -> io::Result<Vec<Hwnd>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_secondary_close_windows()),
        _ => Ok(Vec::new()),
    }
}

/// Takes one pending modal dialog request only from its associated UI session.
pub(crate) fn take_file_dialog_request(window: Hwnd) -> io::Result<Option<FileDialogRequest>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(session.take_file_dialog_request()),
        _ => Ok(None),
    }
}

/// Completes one request from its owning native UI session.
pub(crate) fn complete_file_dialog_request(
    window: Hwnd,
    request_id: u64,
    selection: Result<FileDialogSelection, anodrel_windows_file_dialog::FileDialogError>,
) -> io::Result<Option<bool>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(Some(
            session.complete_file_dialog_request(request_id, selection),
        )),
        _ => Ok(None),
    }
}

/// Takes one pending notification only from its associated UI session.
///
/// The session's existing entry comes back with it so the Shell32 call can run
/// outside this lock.
pub(crate) fn take_notification_request(
    window: Hwnd,
) -> io::Result<
    Option<(
        anodrel_notifications::NotificationRequest,
        Option<std::sync::Arc<anodrel_windows_notification_area::NotificationArea>>,
    )>,
> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(session.take_notification_request()),
        _ => Ok(None),
    }
}

/// Completes one notification from its owning native UI session, recording the
/// entry when this was the session's first.
pub(crate) fn complete_notification_request(
    window: Hwnd,
    request_id: u64,
    shown: bool,
    entry: Option<std::sync::Arc<anodrel_windows_notification_area::NotificationArea>>,
) -> io::Result<Option<bool>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => {
            if let Some(entry) = entry {
                session.set_notification_entry(entry);
            }
            Ok(Some(
                session.complete_notification_request(request_id, shown),
            ))
        }
        _ => Ok(None),
    }
}

/// Takes one pending tray replacement only from its associated UI session.
///
/// The native mapping is built after this registry lock is released, so a
/// Shell32 or User32 failure cannot block other session views.
pub(crate) fn take_tray_request(
    window: Hwnd,
) -> io::Result<
    Option<(
        anodrel_menu::TrayRequest,
        Option<std::sync::Arc<anodrel_windows_notification_area::NotificationArea>>,
    )>,
> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session
            .take_tray_request()
            .map(|request| (request, session.notification_entry()))),
        _ => Ok(None),
    }
}

/// Retains a complete host-built tray and optionally its first shared entry.
pub(crate) fn replace_tray(
    window: Hwnd,
    tray: super::super::tray::TrayMenu,
    entry: Option<std::sync::Arc<anodrel_windows_notification_area::NotificationArea>>,
) -> io::Result<Option<bool>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => {
            if let Some(entry) = entry {
                session.set_notification_entry(entry);
            }
            Ok(Some(session.replace_tray(tray)))
        }
        _ => Ok(None),
    }
}

/// Completes one tray replacement only through its associated view.
pub(crate) fn complete_tray_request(
    window: Hwnd,
    request_id: u64,
    applied: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => {
            Ok(Some(session.complete_tray_request(request_id, applied)))
        }
        _ => Ok(None),
    }
}

/// Returns a current tray mapping for one host-owned local callback.
pub(crate) fn tray(window: Hwnd) -> io::Result<Option<super::super::tray::TrayMenu>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.tray()),
        _ => Ok(None),
    }
}

/// Offers one selected current tray action to the session's shared queue.
pub(crate) fn offer_tray_candidate(
    window: Hwnd,
    candidate: anodrel_ui_session::TrayInputCandidate,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.offer_tray_candidate(candidate))),
        _ => Ok(None),
    }
}

/// Takes one pending menu replacement only from its associated UI session.
///
/// The resulting model has no native object yet, so User32 construction can
/// occur before the view registry is locked again to attach it.
pub(crate) fn take_menu_request(window: Hwnd) -> io::Result<Option<MenuRequest>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_menu_request()),
        _ => Ok(None),
    }
}

/// Takes one pending context-menu replacement only from its associated UI session.
///
/// The resulting model contains no native popup yet, so User32 construction
/// occurs before the view registry is locked again to retain it.
pub(crate) fn take_context_menu_request(window: Hwnd) -> io::Result<Option<ContextMenuRequest>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_context_menu_request()),
        _ => Ok(None),
    }
}

/// Replaces one already constructed context-menu model on its associated view.
pub(crate) fn replace_context_menu(window: Hwnd, menu: ContextMenu) -> io::Result<Option<bool>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.replace_context_menu(menu))),
        _ => Ok(None),
    }
}

/// Completes one context-menu replacement only through its associated view.
pub(crate) fn complete_context_menu_request(
    window: Hwnd,
    request_id: u64,
    applied: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(
            session.complete_context_menu_request(request_id, applied),
        )),
        _ => Ok(None),
    }
}

/// Returns a clone of one current context-menu model for local popup display.
///
/// The native popup call itself runs after this registry lock is released. A
/// later model replacement remains safe because the core revalidates the
/// returned candidate against its revision before it exposes an event.
pub(crate) fn context_menu(window: Hwnd) -> io::Result<Option<ContextMenu>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.context_menu()),
        _ => Ok(None),
    }
}

/// Offers one selected host-private context-menu action to its session queue.
pub(crate) fn offer_context_menu_candidate(
    window: Hwnd,
    candidate: anodrel_ui_session::ContextMenuInputCandidate,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.offer_context_menu_candidate(candidate))),
        _ => Ok(None),
    }
}

/// Attaches one constructed native menu only to its associated UI session.
pub(crate) fn attach_menu(window: Hwnd, menu: UnattachedMenu) -> io::Result<Option<bool>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.attach_menu(window, menu))),
        _ => Ok(None),
    }
}

/// Completes one menu replacement only through its associated UI session.
pub(crate) fn complete_menu_request(
    window: Hwnd,
    request_id: u64,
    applied: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => {
            Ok(Some(session.complete_menu_request(request_id, applied)))
        }
        _ => Ok(None),
    }
}

/// Offers one current private native-menu command to its shared session queue.
///
/// A non-menu `WM_COMMAND`, an accelerator, a control notification, or an
/// unknown/stale numeric ID all answer `false` and retain default processing.
pub(crate) fn offer_menu_command(
    window: Hwnd,
    wparam: usize,
    lparam: isize,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.offer_menu_command(wparam, lparam))),
        _ => Ok(None),
    }
}

/// Offers one current local menu shortcut to its shared session queue.
///
/// The caller has already limited this to one first `WM_KEYDOWN` and copied the
/// current modifier state. No keyboard value crosses the registry boundary.
pub(crate) fn offer_menu_shortcut(
    window: Hwnd,
    key: usize,
    control_down: bool,
    shift_down: bool,
    alt_down: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.offer_menu_shortcut(
            key,
            control_down,
            shift_down,
            alt_down,
        ))),
        _ => Ok(None),
    }
}

/// Takes one pending field read only from its associated UI session.
pub(crate) fn take_field_read(window: Hwnd) -> io::Result<Option<u64>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_field_read()),
        _ => Ok(None),
    }
}

/// Answers one field read from its owning native UI session.
///
/// The snapshot is built while this lock is held, unlike the notification and
/// title calls that are deliberately released first. It is a copy of a handful
/// of short strings with no operating-system call in it, so there is nothing
/// here that a slow system could block every other window behind.
pub(crate) fn complete_field_read(window: Hwnd, request_id: u64) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.complete_field_read(request_id))),
        _ => Ok(None),
    }
}

/// Returns the session-local file registry for the UI thread's capture flow.
pub(crate) fn file_text_service(window: Hwnd) -> io::Result<Option<WindowsFileTextService>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.file_text_service())),
        _ => Ok(None),
    }
}

/// Returns the session-local folder registry for the UI thread's capture flow.
pub(crate) fn folder_entry_service(window: Hwnd) -> io::Result<Option<WindowsFolderEntryService>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.folder_entry_service()),
        _ => Ok(None),
    }
}
