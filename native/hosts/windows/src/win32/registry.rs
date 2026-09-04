//! Per-window view storage for the Win32 message loop.

use std::{
    collections::BTreeMap,
    io,
    sync::{Mutex, OnceLock},
};

use super::{
    Hwnd, StartupLab, View,
    context_menu::ContextMenu,
    menu::UnattachedMenu,
    ui_lab::{AccessibilityFocusResult, AccessibilityScrollResult, UiLab},
    ui_session_view::UiSessionPoll,
};
use anodrel_crash::CrashSurface;
use anodrel_file_dialog::{FileDialogRequest, FileDialogSelection};
use anodrel_menu::{ContextMenuRequest, MenuRequest};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_folder_access::WindowsFolderEntryService;

mod accessibility;
mod session;
mod window_commands;

pub(super) use accessibility::{
    accessibility_snapshot, service_accessibility_focus, service_accessibility_scroll,
};
pub(super) use session::{
    attach_menu, complete_context_menu_request, complete_field_read, complete_file_dialog_request,
    complete_menu_request, complete_notification_request, complete_tray_request, context_menu,
    file_text_service, folder_entry_service, offer_context_menu_candidate, offer_menu_command,
    offer_menu_shortcut, offer_tray_candidate, poll_ui_session, register_ui_session_window,
    replace_context_menu, replace_tray, take_context_menu_request, take_field_read,
    take_file_dialog_request, take_menu_request, take_notification_request,
    take_secondary_close_windows, take_secondary_open_request, take_tray_request, tray,
};
pub(super) use window_commands::{
    complete_window_focus_request, complete_window_fullscreen_request,
    complete_window_size_request, complete_window_state_read_request,
    complete_window_state_request, complete_window_title_request, record_window_state_change,
    set_window_fullscreen_restore, take_window_focus_request, take_window_fullscreen_request,
    take_window_size_request, take_window_state_read_request, take_window_state_request,
    take_window_title_request,
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
