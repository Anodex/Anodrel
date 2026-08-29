//! Host-owned session and operating-system service bridges.
//!
//! Each bridge resolves a window only from the private registry, performs a
//! closed native action on the UI thread, and completes the matching mailbox.
//! Application requests cannot target a window, supply a handle, or read back
//! native state through this module.

use super::*;
mod window_presentation;

#[cfg(test)]
pub(super) use window_presentation::{observed_presentation_state, presentation_command};
pub(super) use window_presentation::{
    record_window_state_change, service_window_focus, service_window_fullscreen,
    service_window_size, service_window_state, service_window_state_read, service_window_title,
};

/// Opens an additional native window while the message loop is running.
pub(super) fn open_document_window(title: &str, document: Document) -> io::Result<()> {
    let instance = module_handle()?;
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    ensure_window_class(instance, &class_name)?;
    let definition = WindowDefinition {
        title: title.to_owned(),
        width: 760,
        height: 560,
        view: View::Document(document),
    };
    let window = create_window(instance, &class_name, &definition)?;
    if let Err(error) = registry::insert(window, definition.view) {
        destroy_window(window);
        return Err(error);
    }
    apply_icons(window);
    show_and_update(window);
    Ok(())
}

/// Opens the native window for one collected product session.
///
/// The window consumes only that session's grouped resources and owns its
/// lifetime: destroying it drops the session, which requests shutdown of the
/// verified child, the pipe worker, and the exit watcher.
pub(super) fn open_product_session_window(
    session: anodrel_windows_product_session::RunningProductSession,
) -> io::Result<()> {
    let instance = module_handle()?;
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    ensure_window_class(instance, &class_name)?;
    let scale = primary_scale();
    let definition = WindowDefinition {
        // Named in full here, where there is room for it. The tile that opens
        // this window has to fit its label into a quarter of a strip.
        title: "Anodrel Development Product Fixture".to_owned(),
        width: (920.0 * scale) as i32,
        height: (660.0 * scale) as i32,
        view: View::UiSession(Box::new(
            ui_session_view::UiSessionView::for_product_session(session),
        )),
    };
    let window = create_window(instance, &class_name, &definition)?;
    if let Err(error) = registry::insert(window, definition.view) {
        destroy_window(window);
        return Err(error);
    }
    let joined_group = match registry::register_ui_session_window(window) {
        Ok(Some(joined)) => joined,
        Ok(None) => false,
        Err(error) => {
            destroy_window(window);
            return Err(error);
        }
    };
    if !joined_group {
        // The group membership must exist before the first window can be
        // shown. Destroying the just-registered view releases the product
        // group, which in turn performs its ordinary verified-child cleanup.
        destroy_window(window);
        return Err(io::Error::other(
            "product session window could not join its native view group",
        ));
    }
    product_tile::note_window(window);
    apply_icons(window);
    // SAFETY: the window was just created on this thread's message queue and
    // its timer stops when the window is destroyed.
    unsafe {
        SetTimer(window, UI_SESSION_TIMER, UI_SESSION_POLL_INTERVAL_MILLIS, 0);
    }
    show_and_update(window);
    Ok(())
}

/// Opens one committed secondary view for an already-known session group.
///
/// This is deliberately private host lifecycle code. The request was produced
/// by the portable group's take-once handoff; it creates, enters both native
/// registries, and commits that handoff before the window is shown. Failure at
/// any earlier point rolls the pending logical view back without exposing a
/// native cause to the waiting worker.
pub(super) fn open_session_secondary_window(
    request: session_window_group::SessionWindowOpenRequest,
) -> io::Result<()> {
    let instance = match module_handle() {
        Ok(instance) => instance,
        Err(error) => {
            let _ = request.fail();
            return Err(error);
        }
    };
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    if let Err(error) = ensure_window_class(instance, &class_name) {
        let _ = request.fail();
        return Err(error);
    }
    let scale = primary_scale();
    let definition = WindowDefinition {
        title: request.caption(),
        width: (920.0 * scale) as i32,
        height: (660.0 * scale) as i32,
        view: View::UiSession(Box::new(ui_session_view::UiSessionView::for_group_member(
            request.resources(),
            request.member(),
        ))),
    };
    let window = match create_window(instance, &class_name, &definition) {
        Ok(window) => window,
        Err(error) => {
            let _ = request.fail();
            return Err(error);
        }
    };
    if let Err(error) = registry::insert(window, definition.view) {
        destroy_window(window);
        let _ = request.fail();
        return Err(error);
    }
    let joined_group = match registry::register_ui_session_window(window) {
        Ok(Some(joined)) => joined,
        Ok(None) => false,
        Err(error) => {
            destroy_window(window);
            let _ = request.fail();
            return Err(error);
        }
    };
    if !joined_group {
        destroy_window(window);
        let _ = request.fail();
        return Err(io::Error::other(
            "secondary session window could not join its native view group",
        ));
    }
    if !request.complete() {
        // The worker timed out or group shutdown won the race. The native
        // window was registered only so WM_DESTROY can remove that exact
        // private mapping; it must never be shown after a failed commit.
        destroy_window(window);
        return Ok(());
    }
    apply_icons(window);
    // SAFETY: this newly created registered view owns its low-frequency
    // session poll; WM_DESTROY stops the timer on the same UI thread.
    unsafe {
        SetTimer(window, UI_SESSION_TIMER, UI_SESSION_POLL_INTERVAL_MILLIS, 0);
    }
    show_and_update(window);
    Ok(())
}

/// Services one pending group-owned secondary creation handoff.
///
/// A timer on any member may reach this function, but the portable group hands
/// a request out only once. The failure path already rolls back and replies to
/// the worker with its safe portable category, so a native creation failure is
/// intentionally not logged or surfaced to an application.
pub(super) fn service_session_window_open(window: Hwnd) {
    let Some(request) = registry::take_secondary_open_request(window).ok().flatten() else {
        return;
    };
    let _ = open_session_secondary_window(request);
}

/// Destroys the host-private secondary windows whose authenticated session
/// requested a close. Logical state is released later by `WM_DESTROY`, after
/// Windows confirms each native view has actually left the group.
pub(super) fn service_session_window_close(window: Hwnd) {
    let Ok(targets) = registry::take_secondary_close_windows(window) else {
        return;
    };
    for target in targets {
        // SAFETY: `target` came from this process's private group mapping. The
        // registry lock has been released, and duplicate close requests have
        // already coalesced in the portable group.
        unsafe { DestroyWindow(target) };
    }
}

/// Maps a window's current layout into hierarchical accessibility elements.
///
/// The semantics come from the same layout the surface draws, so what a screen
/// reader is told cannot drift from what is on screen. A window with no UI
/// document publishes nothing, which is the honest answer for a document or
/// Startup Lab surface. It also carries copied host-owned focus and field values
/// alongside the same layout, while only an authenticated UI session adds the
/// bounded action sink used by an enabled button's Invoke pattern.
pub(super) fn accessible_elements_for(
    window: Hwnd,
) -> anodrel_windows_uia::UiAutomationPublication {
    let rect = client_rect(window);
    let Ok(Some(publication)) =
        registry::accessibility_snapshot(window, rect.width() as f32, rect.height() as f32)
    else {
        return anodrel_windows_uia::UiAutomationPublication::empty();
    };
    let registry::AccessibilityPublication {
        snapshot,
        action_sink,
        focus_route,
        scroll_snapshot,
        scroll_items,
        scroll_route,
        focused,
        field_values,
    } = publication;
    let publication = anodrel_windows_uia::UiAutomationPublication::new(
        anodrel_windows_accessibility::accessible_elements(&snapshot, client_origin(window)),
        field_values,
        focused,
        action_sink,
        focus_route.map(|route| route.for_window(window, WM_ANODREL_UIA_FOCUS)),
    );
    match (scroll_snapshot, scroll_route) {
        (Some(snapshot), Some(route)) => publication.with_scroll(
            snapshot,
            scroll_items,
            route.for_window(window, WM_ANODREL_UIA_SCROLL),
        ),
        _ => publication,
    }
}

/// Raises one best-effort outbound focus notification after a real local move.
///
/// The publication is freshly derived after the view-registry mutation ended,
/// so the UI Automation adapter never observes a mutable view or a registry
/// lock. Its result is intentionally not logged or exposed to an application.
pub(super) fn raise_accessibility_focus_changed(window: Hwnd) {
    anodrel_windows_uia::raise_focus_changed(window, accessible_elements_for(window));
}

/// Raises one best-effort subtree invalidation after accepted document replacement.
pub(super) fn raise_accessibility_structure_changed(window: Hwnd) {
    anodrel_windows_uia::raise_structure_changed(window, accessible_elements_for(window));
}

/// Raises one best-effort live-status notification after a later changed
/// authenticated-session document was applied.
///
/// The UI session poll has compared semantic status data only. The adapter
/// additionally filters the requested ID against this current, visible
/// immutable publication before it makes the Windows call.
pub(super) fn raise_accessibility_live_region_changed(
    window: Hwnd,
    status: &anodrel_ui::ElementId,
) {
    anodrel_windows_uia::raise_live_region_changed(window, accessible_elements_for(window), status);
}

/// Applies a pending host-only UI Automation focus request and repaints only
/// when a current validated target became focused.
pub(super) fn service_accessibility_focus(window: Hwnd) {
    let rect = client_rect(window);
    let outcome =
        registry::service_accessibility_focus(window, rect.width() as f32, rect.height() as f32)
            .ok()
            .flatten();
    if outcome.is_some_and(|outcome| outcome.accepted && outcome.changed) {
        invalidate(window);
        raise_accessibility_focus_changed(window);
    }
}

/// Applies one pending host-only UI Automation scroll request and repaints only
/// when the same retained position used by local input changed.
pub(super) fn service_accessibility_scroll(window: Hwnd) {
    let rect = client_rect(window);
    let outcome =
        registry::service_accessibility_scroll(window, rect.width() as f32, rect.height() as f32)
            .ok()
            .flatten();
    if outcome.is_some_and(|outcome| outcome.accepted && outcome.changed) {
        invalidate(window);
    }
}

/// Locates a window's client area on screen, with its current density.
pub(super) fn client_origin(window: Hwnd) -> anodrel_windows_accessibility::ClientOrigin {
    let mut origin = Point { x: 0, y: 0 };
    // SAFETY: `origin` is writable stack storage and the window belongs to this
    // process; the call converts it in place to screen coordinates.
    unsafe {
        ClientToScreen(window, &mut origin);
    }
    // The layout is already composed at the display's real pixel density, so
    // its logical units are physical ones and need no further scaling.
    anodrel_windows_accessibility::ClientOrigin::new(origin.x, origin.y, 1.0)
}

/// Shows one pending notification for a session window, if it has one.
///
/// The Shell32 call runs outside the window registry's lock, so a slow shell
/// cannot block every other window's message handling. The notification-area
/// entry is created on first use and then reused, because creating one eagerly
/// would put an icon on screen for sessions that never notify.
pub(super) fn service_notification(window: Hwnd) {
    let Ok(Some((request, entry))) = registry::take_notification_request(window) else {
        return;
    };

    let (entry, created) = match entry {
        Some(entry) => (Some(entry), None),
        // Host-owned brand artwork, the same icon the window already carries.
        // An application cannot supply, select, or replace it.
        None => match anodrel_windows_notifications::WindowsNotifications::create(
            window,
            ICONS.get_or_init(appicon::create).0.unwrap_or(0),
        ) {
            Ok(entry) => {
                let entry = std::sync::Arc::new(entry);
                (Some(std::sync::Arc::clone(&entry)), Some(entry))
            }
            Err(_) => (None, None),
        },
    };

    let shown = entry.is_some_and(|entry| {
        anodrel_notifications::NotificationService::show(entry.as_ref(), request.notification())
            .is_ok()
    });
    let _ = registry::complete_notification_request(window, request.id(), shown, created);
}

/// Answers one pending field read for a session window, if it has one.
///
/// Runs on the UI thread beside the other session bridges, because the values
/// belong to the window and a protocol worker never reaches into it. See
/// `docs/UI_FIELDS.md`.
pub(super) fn service_field_read(window: Hwnd) {
    let Ok(Some(request_id)) = registry::take_field_read(window) else {
        return;
    };
    let _ = registry::complete_field_read(window, request_id);
}

/// Routes one typed character to whichever view this window carries.
///
/// Returns `None` when the window has no field-bearing view at all, so the
/// caller can fall through to the default procedure, and `Some(changed)` when a
/// view saw the character — including when it refused it, because a refusal is
/// still this window's answer rather than the system's.
pub(super) fn type_character(window: Hwnd, rect: Rect, character: char) -> Option<bool> {
    let (width, height) = (rect.width() as f32, rect.height() as f32);
    registry::with_ui_lab(window, |lab| lab.type_character(width, height, character))
        .ok()
        .flatten()
        .or_else(|| {
            registry::with_ui_session(window, |session| {
                session.type_character(width, height, character)
            })
            .ok()
            .flatten()
        })
}

/// Routes one editing key the same way.
pub(super) fn edit_focused_field(
    window: Hwnd,
    rect: Rect,
    edit: ui_lab::FieldEdit,
) -> Option<bool> {
    let (width, height) = (rect.width() as f32, rect.height() as f32);
    registry::with_ui_lab(window, |lab| lab.edit_focused_field(width, height, edit))
        .ok()
        .flatten()
        .or_else(|| {
            registry::with_ui_session(window, |session| {
                session.edit_focused_field(width, height, edit)
            })
            .ok()
            .flatten()
        })
}

/// Constructs and attaches one pending session menu on its owning UI thread.
///
/// Native construction happens before taking the process-wide view-registry
/// lock. The short locked attachment either replaces the current bar as one
/// operation or leaves it untouched; every failure becomes the portable
/// service's single safe unavailable outcome.
pub(super) fn service_menu(window: Hwnd) {
    let Ok(Some(request)) = registry::take_menu_request(window) else {
        return;
    };
    let applied = menu::UnattachedMenu::build(&request)
        .and_then(|menu| registry::attach_menu(window, menu).ok().flatten())
        .unwrap_or(false);
    let _ = registry::complete_menu_request(window, request.id(), applied);
}
