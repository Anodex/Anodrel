//! Per-window view storage for the Win32 message loop.

use std::{
    collections::BTreeMap,
    io,
    sync::{Mutex, OnceLock},
};

use super::{
    Hwnd, StartupLab, View,
    menu::UnattachedMenu,
    ui_lab::{AccessibilityFocusResult, AccessibilityScrollResult, UiLab},
    ui_session_view::UiSessionPoll,
};
use anodrel_crash::CrashSurface;
use anodrel_file_dialog::{FileDialogRequest, FileDialogSelection};
use anodrel_menu::MenuRequest;
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_folder_access::WindowsFolderEntryService;

mod window_commands;

pub(super) use window_commands::{
    complete_window_focus_request, complete_window_fullscreen_request,
    complete_window_size_request, complete_window_state_request, complete_window_title_request,
    set_window_fullscreen_restore, take_window_focus_request, take_window_fullscreen_request,
    take_window_size_request, take_window_state_request, take_window_title_request,
};

static VIEWS: OnceLock<Mutex<BTreeMap<Hwnd, View>>> = OnceLock::new();

/// The immutable accessibility data published for one window-message query.
///
/// The focus identifier and field values belong to the same current UI Lab
/// state as the snapshot. The UI Automation adapter filters both against the
/// resulting tree before it reports anything to Windows. Scroll-item IDs are
/// derived from that same view state and identify only host-selected semantic
/// descendants of the one scrolling viewport.
pub(super) struct AccessibilityPublication {
    pub(super) snapshot: anodrel_ui::UiAccessibilitySnapshot,
    pub(super) action_sink: Option<anodrel_windows_uia::UiAutomationActionSink>,
    pub(super) focus_route: Option<anodrel_windows_uia::UiAutomationFocusRoute>,
    pub(super) scroll_snapshot: Option<anodrel_windows_uia::UiAutomationScrollSnapshot>,
    pub(super) scroll_items: Vec<anodrel_ui::ElementId>,
    pub(super) scroll_route: Option<anodrel_windows_uia::UiAutomationScrollRoute>,
    pub(super) focused: Option<anodrel_ui::ElementId>,
    pub(super) field_values: Vec<(anodrel_ui::ElementId, String)>,
}

pub(super) fn insert(window: Hwnd, view: View) -> io::Result<()> {
    let mut views = lock_views()?;
    if views.insert(window, view).is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "window view was already registered",
        ));
    }
    Ok(())
}

pub(super) fn view_for(window: Hwnd) -> io::Result<Option<View>> {
    Ok(lock_views()?.get(&window).cloned())
}

/// Classifies a window for a crash record.
///
/// A view-kind query only: it names the kind of surface and never its title,
/// document, application identity, or handle. It clones nothing, because it is
/// called while the host is shutting down after a contained panic and cloning a
/// view there would be work at the worst possible moment.
///
/// An unregistered window and a poisoned registry both answer
/// [`CrashSurface::Unknown`]. Neither can be allowed to fail: this runs on a
/// path whose whole purpose is to leave evidence behind.
pub(super) fn crash_surface(window: Hwnd) -> CrashSurface {
    let Ok(views) = lock_views() else {
        return CrashSurface::Unknown;
    };
    match views.get(&window) {
        Some(View::StartupLab(_)) => CrashSurface::StartupLab,
        Some(View::Document(_)) => CrashSurface::Document,
        Some(View::UiLab(_)) => CrashSurface::UiLab,
        Some(View::UiSession(_)) => CrashSurface::UiSession,
        None => CrashSurface::Unknown,
    }
}

/// Whether the window's current host-owned view reads system appearance while
/// painting. This is a view-kind query only; it exposes no UI document or
/// application state.
pub(super) fn uses_system_appearance(window: Hwnd) -> io::Result<bool> {
    Ok(matches!(
        lock_views()?.get(&window),
        Some(View::UiLab(_) | View::UiSession(_))
    ))
}

/// Mutates a window's Startup Lab state in place.
///
/// Returns `Ok(None)` when the window is absent or is not a Startup Lab, which
/// is the ordinary case for pointer and timer messages arriving at a document
/// window. Interaction state lives here rather than in the drawing code so that
/// a view stays a single host value per window handle.
pub(super) fn with_startup_lab<R>(
    window: Hwnd,
    change: impl FnOnce(&mut StartupLab) -> R,
) -> io::Result<Option<R>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::StartupLab(lab)) => Ok(Some(change(lab))),
        _ => Ok(None),
    }
}

/// Mutates a window's local native UI diagnostic state in place.
///
/// UI Lab and the explicit developer preview are separate from Startup Lab, so
/// their UI action events cannot inherit Startup Lab's linked host operations.
pub(super) fn with_ui_lab<R>(
    window: Hwnd,
    change: impl FnOnce(&mut UiLab) -> R,
) -> io::Result<Option<R>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiLab(lab)) => Ok(Some(change(lab))),
        _ => Ok(None),
    }
}

/// Polls one session view's explicitly supplied mailbox on the UI thread.
pub(super) fn poll_ui_session(window: Hwnd) -> io::Result<Option<UiSessionPoll>> {
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
pub(super) fn register_ui_session_window(window: Hwnd) -> io::Result<Option<bool>> {
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
pub(super) fn take_secondary_open_request(
    window: Hwnd,
) -> io::Result<Option<super::session_window_group::SessionWindowOpenRequest>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_secondary_open_request()),
        _ => Ok(None),
    }
}

/// Takes host-private native secondary windows requested for close by this
/// session group. The caller must invoke `DestroyWindow` only after this
/// registry lock has been released.
pub(super) fn take_secondary_close_windows(window: Hwnd) -> io::Result<Vec<Hwnd>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_secondary_close_windows()),
        _ => Ok(Vec::new()),
    }
}

/// Takes one pending modal dialog request only from its associated UI session.
pub(super) fn take_file_dialog_request(window: Hwnd) -> io::Result<Option<FileDialogRequest>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(session.take_file_dialog_request()),
        _ => Ok(None),
    }
}

/// Completes one request from its owning native UI session.
pub(super) fn complete_file_dialog_request(
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
pub(super) fn take_notification_request(
    window: Hwnd,
) -> io::Result<
    Option<(
        anodrel_notifications::NotificationRequest,
        Option<std::sync::Arc<anodrel_windows_notifications::WindowsNotifications>>,
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
pub(super) fn complete_notification_request(
    window: Hwnd,
    request_id: u64,
    shown: bool,
    entry: Option<std::sync::Arc<anodrel_windows_notifications::WindowsNotifications>>,
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

/// Takes one pending menu replacement only from its associated UI session.
///
/// The resulting model has no native object yet, so User32 construction can
/// occur before the view registry is locked again to attach it.
pub(super) fn take_menu_request(window: Hwnd) -> io::Result<Option<MenuRequest>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_menu_request()),
        _ => Ok(None),
    }
}

/// Attaches one constructed native menu only to its associated UI session.
pub(super) fn attach_menu(window: Hwnd, menu: UnattachedMenu) -> io::Result<Option<bool>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.attach_menu(window, menu))),
        _ => Ok(None),
    }
}

/// Completes one menu replacement only through its associated UI session.
pub(super) fn complete_menu_request(
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
pub(super) fn offer_menu_command(
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
pub(super) fn offer_menu_shortcut(
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
pub(super) fn take_field_read(window: Hwnd) -> io::Result<Option<u64>> {
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
pub(super) fn complete_field_read(window: Hwnd, request_id: u64) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.complete_field_read(request_id))),
        _ => Ok(None),
    }
}

/// Returns the session-local file registry for the UI thread's capture flow.
pub(super) fn file_text_service(window: Hwnd) -> io::Result<Option<WindowsFileTextService>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.file_text_service())),
        _ => Ok(None),
    }
}

/// Returns the session-local folder registry for the UI thread's capture flow.
pub(super) fn folder_entry_service(window: Hwnd) -> io::Result<Option<WindowsFolderEntryService>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.folder_entry_service()),
        _ => Ok(None),
    }
}

/// Mutates only the explicitly associated native UI session view.
pub(super) fn with_ui_session<R>(
    window: Hwnd,
    change: impl FnOnce(&mut super::ui_session_view::UiSessionView) -> R,
) -> io::Result<Option<R>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(Some(change(session))),
        _ => Ok(None),
    }
}

/// Derives accessibility semantics for whichever view a window carries.
///
/// Only the two views that render a UI document have semantics to publish. A
/// document or Startup Lab window reports none, so assistive technology sees
/// the window itself and nothing inside it.
pub(super) fn accessibility_snapshot(
    window: Hwnd,
    width: f32,
    height: f32,
) -> io::Result<Option<AccessibilityPublication>> {
    let views = lock_views()?;
    Ok(match views.get(&window) {
        // A UI Lab is host-owned diagnostic state. Its local action tiles have
        // no authenticated-session mailbox, so they are readable but
        // intentionally expose no UI Automation Invoke pattern.
        Some(View::UiLab(lab)) => Some(AccessibilityPublication {
            snapshot: lab.accessibility_snapshot(width, height),
            action_sink: None,
            focus_route: Some(lab.accessibility_focus_route(None)),
            scroll_snapshot: lab.accessibility_scroll_snapshot(width, height),
            scroll_items: lab.accessibility_scroll_items(width, height),
            scroll_route: Some(lab.accessibility_scroll_route(None)),
            focused: lab.accessibility_focus(),
            field_values: lab.accessibility_field_values(),
        }),
        Some(View::UiSession(session)) => Some(AccessibilityPublication {
            snapshot: session.lab().accessibility_snapshot(width, height),
            action_sink: session.accessibility_action_sink(),
            focus_route: Some(session.accessibility_focus_route()),
            scroll_snapshot: session.lab().accessibility_scroll_snapshot(width, height),
            scroll_items: session.lab().accessibility_scroll_items(width, height),
            scroll_route: Some(session.accessibility_scroll_route()),
            focused: session.lab().accessibility_focus(),
            field_values: session.lab().accessibility_field_values(),
        }),
        _ => None,
    })
}

/// Takes and revalidates a private UI Automation focus request on one view.
///
/// It is intentionally not a generic focus API: the only caller is the
/// host's payload-free UIA wake message, and it cannot choose a view or target.
pub(super) fn service_accessibility_focus(
    window: Hwnd,
    width: f32,
    height: f32,
) -> io::Result<Option<AccessibilityFocusResult>> {
    let mut views = lock_views()?;
    Ok(match views.get_mut(&window) {
        Some(View::UiLab(lab)) => lab.service_accessibility_focus(None, width, height),
        Some(View::UiSession(session)) => session.service_accessibility_focus(width, height),
        _ => None,
    })
}

/// Takes and revalidates a private UI Automation scroll request on one view.
///
/// The caller is the host's payload-free UIA wake message. Its target and
/// command came from the one active host-owned route, not from a window message.
pub(super) fn service_accessibility_scroll(
    window: Hwnd,
    width: f32,
    height: f32,
) -> io::Result<Option<AccessibilityScrollResult>> {
    let mut views = lock_views()?;
    Ok(match views.get_mut(&window) {
        Some(View::UiLab(lab)) => lab.service_accessibility_scroll(None, width, height),
        Some(View::UiSession(session)) => session.service_accessibility_scroll(width, height),
        _ => None,
    })
}

/// Removes a window and returns the number of host windows that remain.
///
/// The removed view is dropped after the registry lock is released. That
/// matters because dropping a product-session view ends its session: it shuts
/// down the verified child and joins two worker threads. Doing that while
/// holding the process-wide registry lock would block every other window's
/// message handling behind it, and would deadlock outright if a worker ever
/// needed to consult the registry on its way out.
pub(super) fn remove(window: Hwnd) -> io::Result<usize> {
    let (removed, remaining) = {
        let mut views = lock_views()?;
        let removed = views.remove(&window);
        (removed, views.len())
    };
    // A group member must learn about native destruction before its final
    // view drops, but after the registry lock is free. A product group may then
    // release its child and join workers without blocking unrelated messages.
    if let Some(View::UiSession(session)) = &removed {
        session.on_native_destroy(window);
    }
    drop(removed);
    Ok(remaining)
}

/// Drops every remaining view and returns how many there were.
///
/// The message loop normally ends only after the last window is destroyed, so
/// this usually finds nothing. It matters when the loop ends early — a
/// contained panic posts a quit message while windows are still registered.
/// Without this, those views would live in a process-wide static that is never
/// dropped, and a product session among them would keep its child running.
///
/// Views are dropped after the lock is released, for the reason [`remove`]
/// gives.
pub(super) fn clear() -> io::Result<usize> {
    let remaining = {
        let mut views = lock_views()?;
        std::mem::take(&mut *views)
    };
    let count = remaining.len();
    for (window, view) in &remaining {
        if let View::UiSession(session) = view {
            session.on_native_destroy(*window);
        }
    }
    drop(remaining);
    Ok(count)
}

fn lock_views() -> io::Result<std::sync::MutexGuard<'static, BTreeMap<Hwnd, View>>> {
    VIEWS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| io::Error::other("window registry is unavailable"))
}

/// The registry is process-global, so tests that assert on the remaining window
/// count must not overlap with tests that register windows.
#[cfg(test)]
static EXCLUSIVE: Mutex<()> = Mutex::new(());

/// Serializes a test that touches the process-global view registry.
#[cfg(test)]
pub(super) fn tests_exclusive() -> std::sync::MutexGuard<'static, ()> {
    EXCLUSIVE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests;
