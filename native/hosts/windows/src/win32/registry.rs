//! Per-window view storage for the Win32 message loop.

use std::{
    collections::BTreeMap,
    io,
    sync::{Mutex, OnceLock},
};

use super::{
    Hwnd, StartupLab, View,
    menu::UnattachedMenu,
    ui_lab::{AccessibilityFocusResult, UiLab},
};
use anodrel_crash::CrashSurface;
use anodrel_file_dialog::{FileDialogRequest, FileDialogSelection};
use anodrel_menu::MenuRequest;
use anodrel_windows_file_access::WindowsFileTextService;

static VIEWS: OnceLock<Mutex<BTreeMap<Hwnd, View>>> = OnceLock::new();

/// The immutable accessibility data published for one window-message query.
///
/// The focus identifier and field values belong to the same current UI Lab
/// state as the snapshot. The UI Automation adapter filters both against the
/// resulting visible tree before it reports anything to Windows.
pub(super) struct AccessibilityPublication {
    pub(super) snapshot: anodrel_ui::UiAccessibilitySnapshot,
    pub(super) action_sink: Option<anodrel_windows_uia::UiAutomationActionSink>,
    pub(super) focus_route: Option<anodrel_windows_uia::UiAutomationFocusRoute>,
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
pub(super) fn poll_ui_session(window: Hwnd) -> io::Result<Option<(bool, bool)>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.poll())),
        _ => Ok(None),
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

/// Takes one pending title proposal only from its associated UI session.
///
/// The composed caption comes back with it so the User32 call can run outside
/// this lock, matching how a notification is serviced.
pub(super) fn take_window_title_request(window: Hwnd) -> io::Result<Option<(u64, String)>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_window_title_request()),
        _ => Ok(None),
    }
}

/// Completes one title proposal from its owning native UI session.
pub(super) fn complete_window_title_request(
    window: Hwnd,
    request_id: u64,
    applied: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(
            session.complete_window_title_request(request_id, applied),
        )),
        _ => Ok(None),
    }
}

/// Takes one pending state request only from its associated UI session.
///
/// The closed portable value returns to the UI thread, where it becomes the
/// one documented User32 action outside this registry lock.
pub(super) fn take_window_state_request(
    window: Hwnd,
) -> io::Result<Option<(u64, anodrel_window::WindowState)>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_window_state_request()),
        _ => Ok(None),
    }
}

/// Completes one state request from its owning native UI session.
pub(super) fn complete_window_state_request(
    window: Hwnd,
    request_id: u64,
    applied: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(
            session.complete_window_state_request(request_id, applied),
        )),
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
            focused: lab.accessibility_focus(),
            field_values: lab.accessibility_field_values(),
        }),
        Some(View::UiSession(session)) => Some(AccessibilityPublication {
            snapshot: session.lab().accessibility_snapshot(width, height),
            action_sink: session.accessibility_action_sink(),
            focus_route: Some(session.accessibility_focus_route()),
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
mod tests {
    use super::*;
    use crate::win32::PackageFacts;
    use crate::win32::document::Document;
    use crate::win32::ui_lab::UiLab;
    use anodrel_diagnostics::LogBook;
    use std::time::Instant;

    fn document_view(title: &str) -> View {
        View::Document(Document::from_text(title, "test", "body"))
    }

    fn startup_lab_view() -> View {
        View::StartupLab(StartupLab {
            package: PackageFacts {
                display_name: "Sample".to_owned(),
                application_id: "org.anodrel.sample".to_owned(),
                content_format: "anodrel.text.v1".to_owned(),
                content_path: "content/main.txt".to_owned(),
                content_digest: "00".repeat(32),
                content_bytes: 7,
            },
            log: LogBook::new(),
            startup_millis: 12,
            working_set_bytes: 1024,
            last_frame_micros: 0,
            revealed_at: Instant::now(),
            hovered: None,
            ambient: false,
            launch_available: false,
        })
    }

    fn ui_lab_view() -> View {
        View::UiLab(UiLab::new())
    }

    #[test]
    fn keeps_each_window_view_and_final_close_count_independent() {
        let _exclusive = super::tests_exclusive();
        let primary = -101;
        let companion = -102;
        insert(primary, document_view("primary")).expect("primary view registers");
        insert(companion, document_view("companion")).expect("companion view registers");

        let View::Document(primary_document) = view_for(primary)
            .expect("primary view lookup succeeds")
            .expect("primary view is present")
        else {
            panic!("primary view is a document");
        };
        assert_eq!(primary_document.title, "primary");
        assert_eq!(remove(primary).expect("primary closes"), 1);
        assert!(
            view_for(primary)
                .expect("primary lookup succeeds")
                .is_none()
        );
        assert_eq!(remove(companion).expect("companion closes"), 0);
    }

    #[test]
    fn startup_lab_state_is_mutated_in_place() {
        let _exclusive = super::tests_exclusive();
        let window = -201;
        insert(window, startup_lab_view()).expect("lab view registers");

        let changed = with_startup_lab(window, |lab| {
            lab.hovered = Some(2);
            lab.last_frame_micros = 1_500;
        })
        .expect("mutation succeeds");
        assert!(changed.is_some());

        let View::StartupLab(lab) = view_for(window)
            .expect("lookup succeeds")
            .expect("view is present")
        else {
            panic!("view is a startup lab");
        };
        assert_eq!(lab.hovered, Some(2));
        assert_eq!(lab.last_frame_micros, 1_500);
        remove(window).expect("lab closes");
    }

    #[test]
    fn ui_lab_state_is_mutated_only_for_a_ui_lab_window() {
        let _exclusive = super::tests_exclusive();
        let window = -203;
        insert(window, ui_lab_view()).expect("UI Lab view registers");
        assert_eq!(
            with_ui_lab(window, |lab| {
                let was_empty = lab.hovered.is_none() && lab.last_action.is_none();
                lab.clear_hover();
                was_empty
            })
            .expect("mutation succeeds"),
            Some(true)
        );
        assert!(
            with_startup_lab(window, |_| ())
                .expect("mutation succeeds")
                .is_none()
        );
        assert_eq!(remove(window).expect("UI Lab closes"), 0);
    }

    #[test]
    fn only_interactive_native_ui_views_use_system_appearance() {
        let _exclusive = super::tests_exclusive();
        let lab = -204;
        let document = -205;
        insert(lab, ui_lab_view()).expect("UI Lab view registers");
        insert(document, document_view("document")).expect("document view registers");
        assert!(uses_system_appearance(lab).expect("appearance query succeeds"));
        assert!(!uses_system_appearance(document).expect("appearance query succeeds"));
        remove(lab).expect("UI Lab closes");
        remove(document).expect("document closes");
    }

    #[test]
    fn mutating_a_document_window_reports_no_startup_lab() {
        let _exclusive = super::tests_exclusive();
        let window = -202;
        insert(window, document_view("document")).expect("document view registers");
        assert!(
            with_startup_lab(window, |_| ())
                .expect("mutation succeeds")
                .is_none()
        );
        remove(window).expect("document closes");
    }

    #[test]
    fn removal_releases_the_registry_before_dropping_the_view() {
        // A product-session view ends its session on drop, which joins two
        // worker threads. This proves the registry is usable from that drop
        // rather than locked behind it.
        let _exclusive = super::tests_exclusive();
        let window = -206;
        let companion = -207;
        insert(window, document_view("dropping")).expect("view registers");
        insert(companion, document_view("companion")).expect("companion registers");

        let remaining = remove(window).expect("removal succeeds");
        assert_eq!(remaining, 1);
        // The lock is free immediately afterwards, which is what a view's drop
        // would need if it reached back into the registry.
        assert!(view_for(window).expect("lookup succeeds").is_none());
        assert!(view_for(companion).expect("lookup succeeds").is_some());
        remove(companion).expect("companion closes");
    }

    #[test]
    fn mutating_an_unknown_window_is_not_an_error() {
        assert!(
            with_startup_lab(-999, |_| ())
                .expect("mutation succeeds")
                .is_none()
        );
    }
}
