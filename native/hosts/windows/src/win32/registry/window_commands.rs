//! Session-owned window-command access through the host registry.

use std::io;

use super::*;

/// Takes one pending title proposal only from its associated UI session.
pub(crate) fn take_window_title_request(window: Hwnd) -> io::Result<Option<(u64, String)>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_window_title_request()),
        _ => Ok(None),
    }
}

/// Completes one title proposal from its owning native UI session.
pub(crate) fn complete_window_title_request(
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
pub(crate) fn take_window_state_request(
    window: Hwnd,
) -> io::Result<Option<(u64, anodrel_window::WindowState)>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_window_state_request()),
        _ => Ok(None),
    }
}

/// Completes one state request from its owning native UI session.
pub(crate) fn complete_window_state_request(
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

/// Takes one pending pull-only state observation from its associated UI session.
pub(crate) fn take_window_state_read_request(window: Hwnd) -> io::Result<Option<u64>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_window_state_read_request()),
        _ => Ok(None),
    }
}

/// Completes one state observation through its owning native UI session.
pub(crate) fn complete_window_state_read_request(
    window: Hwnd,
    request_id: u64,
    state: Option<anodrel_window::WindowState>,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(
            session.complete_window_state_read_request(request_id, state),
        )),
        _ => Ok(None),
    }
}

/// Records one observed state only in its associated UI session mailbox.
pub(crate) fn record_window_state_change(
    window: Hwnd,
    state: anodrel_window::WindowState,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(session.record_window_state_change(state))),
        _ => Ok(None),
    }
}

/// Takes one pending foreground request only from its associated UI session.
pub(crate) fn take_window_focus_request(window: Hwnd) -> io::Result<Option<u64>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session.take_window_focus_request()),
        _ => Ok(None),
    }
}

/// Completes one foreground request from its owning native UI session.
pub(crate) fn complete_window_focus_request(
    window: Hwnd,
    request_id: u64,
    requested: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(
            session.complete_window_focus_request(request_id, requested),
        )),
        _ => Ok(None),
    }
}

/// Takes one pending fullscreen mode only from its associated UI session.
pub(crate) fn take_window_fullscreen_request(
    window: Hwnd,
) -> io::Result<
    Option<(
        u64,
        anodrel_window::WindowFullscreenMode,
        Option<super::super::fullscreen::FullscreenRestore>,
    )>,
> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session
            .take_window_fullscreen_request()
            .map(|(request_id, mode)| (request_id, mode, session.fullscreen_restore()))),
        _ => Ok(None),
    }
}

/// Stores or clears private fullscreen restoration facts for one UI session.
pub(crate) fn set_window_fullscreen_restore(
    window: Hwnd,
    restore: Option<super::super::fullscreen::FullscreenRestore>,
) -> io::Result<Option<()>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::UiSession(session)) => {
            session.set_fullscreen_restore(restore);
            Ok(Some(()))
        }
        _ => Ok(None),
    }
}

/// Completes one fullscreen request from its owning native UI session.
pub(crate) fn complete_window_fullscreen_request(
    window: Hwnd,
    request_id: u64,
    applied: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(
            session.complete_window_fullscreen_request(request_id, applied),
        )),
        _ => Ok(None),
    }
}

/// Takes one pending bounded client-size request from its associated session.
pub(crate) fn take_window_size_request(
    window: Hwnd,
) -> io::Result<Option<(u64, anodrel_window::WindowSize, bool)>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(session
            .take_window_size_request()
            .map(|(request_id, size)| (request_id, size, session.fullscreen_restore().is_some()))),
        _ => Ok(None),
    }
}

/// Completes one client-size request through its owning native UI session.
pub(crate) fn complete_window_size_request(
    window: Hwnd,
    request_id: u64,
    applied: bool,
) -> io::Result<Option<bool>> {
    let views = lock_views()?;
    match views.get(&window) {
        Some(View::UiSession(session)) => Ok(Some(
            session.complete_window_size_request(request_id, applied),
        )),
        _ => Ok(None),
    }
}
