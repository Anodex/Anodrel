//! Development UI-session diagnostic entry points.
//!
//! Each route fixes one client request sequence. The parent module owns the
//! private transport, host-service composition, child lifetime, and UI thread.

use std::error::Error;

use super::{request::SampleDialogRequest, run_ui_session_with_dialog};

/// Runs the development bootstrap sample while one native window consumes its
/// authenticated document mailbox.
pub fn run_ui_session(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::None)
}

/// Runs the UI-session diagnostic through two visible v3 live-status updates.
pub fn run_ui_session_with_live_status(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::LiveStatus)
}

/// Runs the UI session diagnostic and asks its client to show one open picker.
pub fn run_ui_session_with_open_file_dialog(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::OpenFile)
}

/// Runs the UI session diagnostic and asks its client to show one folder picker.
pub fn run_ui_session_with_open_folder_dialog(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::OpenFolder)
}

/// Runs the UI session diagnostic through one captured folder snapshot.
pub fn run_ui_session_with_selected_folder_entries(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(
        node_path,
        client_path,
        SampleDialogRequest::OpenFolderWithReference,
    )
}

/// Runs the UI session diagnostic through one selection-scoped text read.
pub fn run_ui_session_with_selected_file_text(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(
        node_path,
        client_path,
        SampleDialogRequest::OpenFileWithReference,
    )
}

/// Runs the UI session diagnostic and asks its client to show one save picker.
pub fn run_ui_session_with_save_file_dialog(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::SaveFile)
}

/// Runs the UI-session diagnostic through one selection-scoped native text write.
pub fn run_ui_session_with_selected_file_write(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(
        node_path,
        client_path,
        SampleDialogRequest::SaveFileWithReference,
    )
}

/// Runs the UI-session diagnostic through one selection-scoped native binary write.
pub fn run_ui_session_with_selected_binary_file_write(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(
        node_path,
        client_path,
        SampleDialogRequest::SaveFileBinaryWithReference,
    )
}

/// Runs the UI session diagnostic and asks its client to replace and read state.
pub fn run_ui_session_with_storage(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Storage)
}

/// Runs the UI session diagnostic and asks its client to submit a scroll tree.
pub fn run_ui_session_with_scroll(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Scroll)
}

/// Runs the UI session diagnostic and asks its client to read host diagnostics.
pub fn run_ui_session_with_diagnostics(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Diagnostics)
}

/// Runs the UI-session diagnostic against the application's Credential Manager namespace.
pub fn run_ui_session_with_credentials(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Credentials)
}

/// Runs the UI-session diagnostic and asks its client to show one notification.
pub fn run_ui_session_with_notification(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Notification)
}

/// Runs the UI session diagnostic and asks its client to propose a window title.
pub fn run_ui_session_with_window_title(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowTitle)
}

/// Runs the UI-session diagnostic through minimise, maximise, and restore.
pub fn run_ui_session_with_window_state(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowState)
}

/// Runs the UI-session diagnostic through a state snapshot before and after changes.
pub fn run_ui_session_with_window_state_read(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowStateRead)
}

/// Runs the UI-session diagnostic through coalesced state-change reads.
pub fn run_ui_session_with_window_state_changes(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(
        node_path,
        client_path,
        SampleDialogRequest::WindowStateChanges,
    )
}

/// Runs the UI-session diagnostic through one guarded foreground request.
pub fn run_ui_session_with_window_focus(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowFocus)
}

/// Runs the UI-session diagnostic through reversible borderless fullscreen.
pub fn run_ui_session_with_window_fullscreen(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(
        node_path,
        client_path,
        SampleDialogRequest::WindowFullscreen,
    )
}

/// Runs the UI-session diagnostic through one bounded client-area resize.
pub fn run_ui_session_with_window_size(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowSize)
}

/// Runs the UI-session diagnostic through a fullscreen size refusal.
pub fn run_ui_session_with_window_size_while_fullscreen(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(
        node_path,
        client_path,
        SampleDialogRequest::WindowSizeWhileFullscreen,
    )
}

/// Runs the UI session diagnostic with two fields a person can type into.
pub fn run_ui_session_with_field_read(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::FieldRead)
}

/// Runs the UI session diagnostic through one direct host-owned native menu.
pub fn run_ui_session_with_menu(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Menu)
}
