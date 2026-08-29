//! Development-only end-to-end sample orchestration.
//!
//! This path proves the private bootstrap and named-pipe contract. It does not
//! verify the child executable or provide product application lifecycle policy.

mod request;

use std::{error::Error, io, thread};

use anodrel_application::ApplicationManifest;
use anodrel_core::HostPolicy;
use anodrel_diagnostics::{Event, LogBook};
use anodrel_file_access::SelectionFileDialogMailbox;
use anodrel_folder_access::FolderFileDialogMailbox;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_clipboard::WindowsClipboard;
use anodrel_windows_credentials::WindowsCredentialService;
use anodrel_windows_external_links::WindowsExternalLinks;
use anodrel_windows_paths::application_directories;
use anodrel_windows_pipe::WindowsPipeServer;
use anodrel_windows_storage::WindowsStorageService;

use crate::session_ui::DevelopmentSessionUi;
use request::{SampleDialogRequest, sample_capabilities};

const SAMPLE_TIMEOUT_MILLISECONDS: u32 = 10_000;

pub fn run(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_with_optional_session_view(node_path, client_path, None, SampleDialogRequest::None)
}

/// Runs the development bootstrap sample while one native window consumes its
/// authenticated document mailbox.
pub fn run_ui_session(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::None)
}

/// Runs the UI-session diagnostic through two visible v3 live-status updates.
///
/// The existing document-write and semantic-action grants are sufficient. The
/// child cannot discover whether Windows announced either update.
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

/// Runs the UI session diagnostic and asks its client for a selection-scoped
/// text read through the native UI-thread capture path.
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

/// Runs the UI-session diagnostic through one selection-scoped native text
/// replacement. The client receives a save reference, not output authority
/// that can be redirected to a later path.
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

/// Runs the UI-session diagnostic through one selection-scoped native binary
/// replacement. The client receives only a one-use save reference, not a path
/// or a reusable binary file handle.
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

/// Runs the UI session diagnostic and asks its client to read closed host diagnostics.
pub fn run_ui_session_with_diagnostics(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Diagnostics)
}

/// Runs the UI-session diagnostic against the current user's exact
/// application-bound Credential Manager namespace.
pub fn run_ui_session_with_credentials(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Credentials)
}

/// Runs the UI-session diagnostic and asks its client to show one notification.
///
/// The client learns only that the host accepted the values. Whether the
/// notification appeared is for the operator to observe on screen.
pub fn run_ui_session_with_notification(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Notification)
}

/// Runs the UI session diagnostic and asks its client to propose a window title.
///
/// The client deliberately proposes a name that would be a lie on its own, so
/// the visible result is the composition rule working: the caption reads
/// `Windows Security — Anodrel Sample`. See `docs/WINDOW_TITLE.md`.
pub fn run_ui_session_with_window_title(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowTitle)
}

/// Runs the UI-session diagnostic through minimise, maximise, and restore.
///
/// The client receives acceptance only; an operator observes the visible state
/// changes. See `docs/WINDOW_STATE.md`.
pub fn run_ui_session_with_window_state(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowState)
}

/// Runs the UI-session diagnostic through state observations before and after
/// its own closed state commands.
pub fn run_ui_session_with_window_state_read(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowStateRead)
}

/// Runs the UI-session diagnostic through one guarded foreground request.
///
/// An operator brings another application forward during the short client
/// delay, then observes Windows' own foreground-policy outcome. See
/// `docs/WINDOW_FOCUS.md`.
pub fn run_ui_session_with_window_focus(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowFocus)
}

/// Runs the UI-session diagnostic through reversible borderless fullscreen.
///
/// The client requests fullscreen and then windowed mode. An operator observes
/// both the monitor-filling transition and restoration; it receives no monitor
/// or bounds readback. See `docs/WINDOW_FULLSCREEN.md`.
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
///
/// The client receives acceptance only. An operator observes that the session
/// window resizes without moving or activating; see `docs/WINDOW_SIZE.md`.
pub fn run_ui_session_with_window_size(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::WindowSize)
}

/// Runs the UI-session diagnostic through a fullscreen size refusal.
///
/// The client enters reversible fullscreen, verifies that a client-size request
/// safely fails, and then restores the host-retained presentation.
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
///
/// The client reads the field values twice: once before anyone has typed, and
/// once after the semantic action. The difference between those two reads is
/// the whole of what an application learns, and it learns it only because it
/// asked. See `docs/UI_FIELDS.md` and Decision 0067.
pub fn run_ui_session_with_field_read(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::FieldRead)
}

/// Runs the UI session diagnostic through one direct host-owned native menu.
///
/// The client publishes one semantic command and exits only after a person
/// selects it from the real User32 bar and receives the bounded pull event.
pub fn run_ui_session_with_menu(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::Menu)
}

/// The name the sample host appends to any title the sample proposes.
///
/// Host-chosen, exactly like a registered session's validated display name is
/// host-held: what matters for the guarantee is that the application cannot
/// influence it, not where the host got it.
const SAMPLE_DISPLAY_NAME: &str = "Anodrel Sample";

fn run_ui_session_with_dialog(
    node_path: &str,
    client_path: &str,
    dialog_request: SampleDialogRequest,
) -> Result<(), Box<dyn Error>> {
    run_with_optional_session_view(
        node_path,
        client_path,
        Some(DevelopmentSessionUi::new()),
        dialog_request,
    )
}

fn run_with_optional_session_view(
    node_path: &str,
    client_path: &str,
    session_ui: Option<DevelopmentSessionUi>,
    dialog_request: SampleDialogRequest,
) -> Result<(), Box<dyn Error>> {
    let policy = HostPolicy::new(
        "anodrel.sample",
        sample_capabilities(dialog_request),
        "anodrel-windows-host",
    )?;
    let (server, invitation) = match session_ui.as_ref() {
        Some(ui) => {
            // Composing the bundle keeps every service named at its own call
            // rather than positionally in one ever-growing constructor.
            let services = anodrel_core::HostServices::unavailable()
                .with_clipboard(WindowsClipboard::new(0))
                .with_external_links(WindowsExternalLinks)
                .with_file_dialogs(ui.file_dialog.clone())
                .with_file_selections(SelectionFileDialogMailbox::new(ui.file_dialog.clone()))
                .with_file_text(ui.file_text.clone())
                .with_folder_selections(FolderFileDialogMailbox::new(ui.file_dialog.clone()))
                .with_folder_entries(ui.folder_entries.clone())
                .with_file_save_selections(anodrel_file_access::SaveFileDialogMailbox::new(
                    ui.file_dialog.clone(),
                ))
                .with_file_text_write(ui.file_text.write_service())
                .with_file_binary_write(ui.file_text.binary_write_service())
                .with_storage(sample_storage()?)
                .with_diagnostics(sample_diagnostics())
                .with_credentials(sample_credentials()?)
                .with_notifications(ui.notifications.clone())
                .with_menu(ui.menu.clone())
                .with_window_title(ui.window_title.clone())
                .with_window_state(ui.window_state.clone())
                .with_window_state_read(ui.window_state_read.clone())
                .with_window_focus(ui.window_focus.clone())
                .with_window_fullscreen(ui.window_fullscreen.clone())
                .with_window_size(ui.window_size.clone())
                .with_ui_fields(ui.fields.clone());
            WindowsPipeServer::create_with_session_components_and_service_bundle(
                policy,
                "sample-session",
                ui.document.clone(),
                ui.input.clone(),
                ui.close.clone(),
                services,
            )?
        }
        None => WindowsPipeServer::create(policy, "sample-session")?,
    };
    let bootstrap = invitation.bootstrap_invitation()?;
    let server_thread = thread::spawn(move || server.serve_one());

    let command = if session_ui.is_some() {
        let command = BootstrapCommand::new(node_path)?
            .arg(client_path)?
            .arg("--wait-for-ui-event")?;
        match dialog_request {
            SampleDialogRequest::None => command,
            SampleDialogRequest::OpenFile => command.arg("--request-open-file")?,
            SampleDialogRequest::OpenFolder => command.arg("--request-open-folder")?,
            SampleDialogRequest::OpenFolderWithReference => {
                command.arg("--request-selected-folder-entries")?
            }
            SampleDialogRequest::OpenFileWithReference => {
                command.arg("--request-selected-file-text")?
            }
            SampleDialogRequest::SaveFile => command.arg("--request-save-file")?,
            SampleDialogRequest::SaveFileWithReference => {
                command.arg("--request-save-file-text")?
            }
            SampleDialogRequest::SaveFileBinaryWithReference => {
                command.arg("--request-save-file-binary")?
            }
            SampleDialogRequest::Storage => command.arg("--request-storage-state")?,
            SampleDialogRequest::Scroll => command.arg("--request-scroll-document")?,
            SampleDialogRequest::Diagnostics => command.arg("--request-diagnostics")?,
            SampleDialogRequest::Credentials => command.arg("--request-credentials")?,
            SampleDialogRequest::Notification => command.arg("--request-notification")?,
            SampleDialogRequest::WindowTitle => command.arg("--request-window-title")?,
            SampleDialogRequest::WindowState => command.arg("--request-window-state")?,
            SampleDialogRequest::WindowStateRead => command.arg("--request-window-state-read")?,
            SampleDialogRequest::WindowFocus => command.arg("--request-window-focus")?,
            SampleDialogRequest::WindowFullscreen => command.arg("--request-window-fullscreen")?,
            SampleDialogRequest::WindowSize => command.arg("--request-window-size")?,
            SampleDialogRequest::WindowSizeWhileFullscreen => {
                command.arg("--request-window-size-while-fullscreen")?
            }
            SampleDialogRequest::FieldRead => command.arg("--request-field-read")?,
            SampleDialogRequest::Menu => command.arg("--request-native-menu")?,
            SampleDialogRequest::LiveStatus => command.arg("--request-live-status")?,
        }
    } else {
        BootstrapCommand::new(node_path)?.arg(client_path)?
    };
    let child = launch(&command, &bootstrap)?;
    if let Some(ui) = session_ui {
        crate::win32::run_ui_session(
            ui.document,
            ui.input,
            ui.close,
            ui.file_dialog,
            ui.file_text,
            ui.folder_entries,
            ui.notifications,
            ui.menu,
            ui.window_title,
            ui.window_state,
            ui.window_state_read,
            ui.window_focus,
            ui.window_fullscreen,
            ui.window_size,
            SAMPLE_DISPLAY_NAME,
            ui.fields,
        )?;
    }
    let exit_code = child.wait_for_exit(SAMPLE_TIMEOUT_MILLISECONDS)?;
    if exit_code != 0 {
        // The child output is intentionally unavailable so a bootstrap failure
        // cannot accidentally reveal credentials in a host diagnostic.
        return Err(io::Error::other(format!(
            "development sample client failed at safe stage {exit_code}"
        ))
        .into());
    }

    server_thread
        .join()
        .map_err(|_| io::Error::other("development pipe worker panicked"))??;
    println!("Anodrel Windows development sample completed successfully.");
    Ok(())
}

fn sample_storage() -> Result<WindowsStorageService, Box<dyn Error>> {
    let manifest = ApplicationManifest::parse(
        r#"{"manifestVersion":{"major":1,"minor":0},"applicationId":"anodrel.sample","displayName":"Anodrel Sample","content":{"format":"anodrel.text.v1","path":"content/main.txt","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}}"#,
    )?;
    Ok(WindowsStorageService::new(&application_directories(
        manifest.identity(),
    )?))
}

fn sample_credentials() -> Result<WindowsCredentialService, Box<dyn Error>> {
    let manifest = ApplicationManifest::parse(
        r#"{"manifestVersion":{"major":1,"minor":0},"applicationId":"anodrel.sample","displayName":"Anodrel Sample","content":{"format":"anodrel.text.v1","path":"content/main.txt","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}}"#,
    )?;
    Ok(WindowsCredentialService::new(manifest.identity().clone()))
}

fn sample_diagnostics() -> LogBook {
    let mut log = LogBook::new();
    log.record(Event::CoreHealthChecked)
        .expect("the fixed development log cannot exhaust its sequence");
    log.record(Event::PipeLoopbackChecked)
        .expect("the fixed development log cannot exhaust its sequence");
    log
}
