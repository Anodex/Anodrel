//! Host-only launch routes for authenticated native UI session windows.
//!
//! These functions compose the known session mailboxes into one fixed native
//! window. They are not application window constructors: callers cannot select
//! a handle, view, geometry, or another session's resources.

#![allow(clippy::too_many_arguments)]

use super::*;

/// Opens one host-controlled native view that consumes exactly one authenticated
/// session's mailboxes. Actions enter only the bounded semantic-input mailbox
/// and remain incapable of native operations in this diagnostic.
pub fn run_ui_session(
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    folder_entries: WindowsFolderEntryService,
    notifications: NotificationMailbox,
    menu: MenuMailbox,
    window_title: WindowTitleMailbox,
    window_state: WindowStateMailbox,
    window_focus: WindowFocusMailbox,
    window_fullscreen: WindowFullscreenMailbox,
    window_size: WindowSizeMailbox,
    display_name: &str,
    field_reads: UiFieldMailbox,
) -> io::Result<()> {
    run_authenticated_ui_session(
        "Anodrel UI Session Lab",
        mailbox,
        input_mailbox,
        close_signal,
        file_dialog_mailbox,
        file_text,
        folder_entries,
        notifications,
        menu,
        window_title,
        window_state,
        window_focus,
        window_fullscreen,
        window_size,
        display_name,
        field_reads,
    )
}

/// Opens the fixed development session and starts one host-selected observer.
///
/// This stays inside Anodrel's development host. The callback receives only
/// the newly-created host window so a private acceptance worker can attach; no
/// application, protocol, or SDK surface can provide or obtain that handle.
pub(crate) fn run_ui_session_after_shown<F>(
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    folder_entries: WindowsFolderEntryService,
    notifications: NotificationMailbox,
    menu: MenuMailbox,
    window_title: WindowTitleMailbox,
    window_state: WindowStateMailbox,
    window_focus: WindowFocusMailbox,
    window_fullscreen: WindowFullscreenMailbox,
    window_size: WindowSizeMailbox,
    display_name: &str,
    field_reads: UiFieldMailbox,
    after_shown: F,
) -> io::Result<()>
where
    F: FnOnce(Hwnd) -> io::Result<()>,
{
    run_authenticated_ui_session_after_shown(
        "Anodrel UI Session Lab",
        mailbox,
        input_mailbox,
        close_signal,
        file_dialog_mailbox,
        file_text,
        folder_entries,
        notifications,
        menu,
        window_title,
        window_state,
        window_focus,
        window_fullscreen,
        window_size,
        display_name,
        field_reads,
        after_shown,
    )
}

/// Opens one host-selected authenticated application session window.
///
/// The caller must supply resources created together for one already
/// authenticated session. This is host lifecycle code, not an application
/// window-management API: the application cannot create a window, pass a
/// handle, or attach a different session's resource.
///
/// `title` is the caption the host opens with. A session holding the
/// `window.title` grant may later propose a replacement, which the host
/// composes with `display_name` before applying — the application supplies one
/// half and never the other. See `docs/WINDOW_TITLE.md`.
///
/// `window_state` carries only minimise, maximise, and restore requests for
/// this same host-selected window. It is a closed command bridge, not a native
/// handle or a window-management API; see `docs/WINDOW_STATE.md`.
///
/// `window_focus` carries only a request to foreground this same host-selected
/// window. It exposes no target, input, retry, or observed focus state; see
/// `docs/WINDOW_FOCUS.md`.
///
/// `window_fullscreen` carries only a reversible borderless or windowed mode
/// for this same host-selected window. The UI thread retains native restoration
/// facts privately; see `docs/WINDOW_FULLSCREEN.md`.
///
/// `window_size` carries only bounded logical client dimensions for this same
/// host-selected window. The UI thread derives its native frame privately; see
/// `docs/WINDOW_SIZE.md`.
pub fn run_authenticated_ui_session(
    title: &str,
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    folder_entries: WindowsFolderEntryService,
    notifications: NotificationMailbox,
    menu: MenuMailbox,
    window_title: WindowTitleMailbox,
    window_state: WindowStateMailbox,
    window_focus: WindowFocusMailbox,
    window_fullscreen: WindowFullscreenMailbox,
    window_size: WindowSizeMailbox,
    display_name: &str,
    field_reads: UiFieldMailbox,
) -> io::Result<()> {
    run_authenticated_ui_session_after_shown(
        title,
        mailbox,
        input_mailbox,
        close_signal,
        file_dialog_mailbox,
        file_text,
        folder_entries,
        notifications,
        menu,
        window_title,
        window_state,
        window_focus,
        window_fullscreen,
        window_size,
        display_name,
        field_reads,
        |_| Ok(()),
    )
}

fn run_authenticated_ui_session_after_shown<F>(
    title: &str,
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    folder_entries: WindowsFolderEntryService,
    notifications: NotificationMailbox,
    menu: MenuMailbox,
    window_title: WindowTitleMailbox,
    window_state: WindowStateMailbox,
    window_focus: WindowFocusMailbox,
    window_fullscreen: WindowFullscreenMailbox,
    window_size: WindowSizeMailbox,
    display_name: &str,
    field_reads: UiFieldMailbox,
    after_shown: F,
) -> io::Result<()>
where
    F: FnOnce(Hwnd) -> io::Result<()>,
{
    let scale = primary_scale();
    launch::run_windows_after_shown(
        vec![WindowDefinition {
            title: title.to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiSession(Box::new(
                ui_session_view::UiSessionView::new(
                    mailbox,
                    input_mailbox,
                    close_signal,
                    file_dialog_mailbox,
                    file_text,
                    notifications,
                )
                .with_folder_entries(folder_entries)
                .with_menu(menu)
                .with_window_title(window_title, display_name)
                .with_window_state(window_state)
                .with_window_focus(window_focus)
                .with_window_fullscreen(window_fullscreen)
                .with_window_size(window_size)
                .with_field_reads(field_reads),
            )),
        }],
        None,
        move |windows| after_shown(windows[0]),
    )
}
