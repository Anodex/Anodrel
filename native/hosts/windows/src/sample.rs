//! Development-only end-to-end sample orchestration.
//!
//! This path proves the private bootstrap and named-pipe contract. It does not
//! verify the child executable or provide product application lifecycle policy.

mod request;
mod ui_session;

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

pub use ui_session::*;

const SAMPLE_TIMEOUT_MILLISECONDS: u32 = 10_000;

pub fn run(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_with_optional_session_view(node_path, client_path, None, SampleDialogRequest::None)
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
                .with_context_menu(ui.context_menu.clone())
                .with_tray(ui.tray.clone())
                .with_window_title(ui.window_title.clone())
                .with_window_state(ui.window_state.clone())
                .with_window_state_read(ui.window_state_read.clone())
                .with_window_state_changes(ui.window_state_changes.clone())
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
            SampleDialogRequest::WindowStateChanges => {
                command.arg("--request-window-state-changes")?
            }
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
            ui.context_menu,
            ui.tray,
            ui.window_title,
            ui.window_state,
            ui.window_state_read,
            ui.window_state_changes,
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
