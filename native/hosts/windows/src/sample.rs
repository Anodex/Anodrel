//! Development-only end-to-end sample orchestration.
//!
//! This path proves the private bootstrap and named-pipe contract. It does not
//! verify the child executable or provide product application lifecycle policy.

use std::{error::Error, io, thread};

use anodrel_application::ApplicationManifest;
use anodrel_core::{HostPolicy, SessionCloseSignal};
use anodrel_diagnostics::{Event, LogBook};
use anodrel_file_access::SelectionFileDialogMailbox;
use anodrel_file_dialog::FileDialogMailbox;
use anodrel_protocol::Capability;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_clipboard::WindowsClipboard;
use anodrel_windows_credentials::WindowsCredentialService;
use anodrel_windows_external_links::WindowsExternalLinks;
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_paths::application_directories;
use anodrel_windows_pipe::WindowsPipeServer;
use anodrel_windows_storage::WindowsStorageService;

const SAMPLE_TIMEOUT_MILLISECONDS: u32 = 10_000;

#[derive(Clone, Copy)]
enum SampleDialogRequest {
    None,
    OpenFile,
    OpenFileWithReference,
    SaveFile,
    Storage,
    Scroll,
    Diagnostics,
    Credentials,
    Notification,
}

pub fn run(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_with_optional_session_view(node_path, client_path, None, SampleDialogRequest::None)
}

/// Runs the development bootstrap sample while one native window consumes its
/// authenticated document mailbox.
pub fn run_ui_session(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::None)
}

/// Runs the UI session diagnostic and asks its client to show one open picker.
pub fn run_ui_session_with_open_file_dialog(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_dialog(node_path, client_path, SampleDialogRequest::OpenFile)
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

fn run_ui_session_with_dialog(
    node_path: &str,
    client_path: &str,
    dialog_request: SampleDialogRequest,
) -> Result<(), Box<dyn Error>> {
    let mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let file_dialog_mailbox = FileDialogMailbox::new();
    let file_text = WindowsFileTextService::new();
    let notifications = anodrel_notifications::NotificationMailbox::new();
    run_with_optional_session_view(
        node_path,
        client_path,
        Some((
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
            notifications,
        )),
        dialog_request,
    )
}

fn run_with_optional_session_view(
    node_path: &str,
    client_path: &str,
    mailboxes: Option<(
        UiDocumentMailbox,
        UiInputMailbox,
        SessionCloseSignal,
        FileDialogMailbox,
        WindowsFileTextService,
        anodrel_notifications::NotificationMailbox,
    )>,
    dialog_request: SampleDialogRequest,
) -> Result<(), Box<dyn Error>> {
    let policy = HostPolicy::new(
        "anodrel.sample",
        vec![
            Capability::DiagnosticsRead,
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::SessionClose,
            Capability::ClipboardRead,
            Capability::ClipboardWrite,
            Capability::ExternalOpen,
            Capability::DialogOpenFile,
            Capability::DialogSaveFile,
            Capability::FileReadText,
            Capability::StorageStateRead,
            Capability::StorageStateReplace,
            Capability::StorageStateClear,
            Capability::CredentialRead,
            Capability::CredentialWrite,
            Capability::CredentialDelete,
            Capability::NotificationShow,
        ],
        "anodrel-windows-host",
    )?;
    let (server, invitation) = match mailboxes.as_ref() {
        Some((
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
            notifications,
        )) => {
            // Composing the bundle keeps every service named at its own call
            // rather than positionally in one ever-growing constructor.
            let services = anodrel_core::HostServices::unavailable()
                .with_clipboard(WindowsClipboard::new(0))
                .with_external_links(WindowsExternalLinks)
                .with_file_dialogs(file_dialog_mailbox.clone())
                .with_file_selections(SelectionFileDialogMailbox::new(file_dialog_mailbox.clone()))
                .with_file_text(file_text.clone())
                .with_storage(sample_storage()?)
                .with_diagnostics(sample_diagnostics())
                .with_credentials(sample_credentials()?)
                .with_notifications(notifications.clone());
            WindowsPipeServer::create_with_session_components_and_service_bundle(
                policy,
                "sample-session",
                mailbox.clone(),
                input_mailbox.clone(),
                close_signal.clone(),
                services,
            )?
        }
        None => WindowsPipeServer::create(policy, "sample-session")?,
    };
    let bootstrap = invitation.bootstrap_invitation()?;
    let server_thread = thread::spawn(move || server.serve_one());

    let command = if mailboxes.is_some() {
        let command = BootstrapCommand::new(node_path)?
            .arg(client_path)?
            .arg("--wait-for-ui-event")?;
        match dialog_request {
            SampleDialogRequest::None => command,
            SampleDialogRequest::OpenFile => command.arg("--request-open-file")?,
            SampleDialogRequest::OpenFileWithReference => {
                command.arg("--request-selected-file-text")?
            }
            SampleDialogRequest::SaveFile => command.arg("--request-save-file")?,
            SampleDialogRequest::Storage => command.arg("--request-storage-state")?,
            SampleDialogRequest::Scroll => command.arg("--request-scroll-document")?,
            SampleDialogRequest::Diagnostics => command.arg("--request-diagnostics")?,
            SampleDialogRequest::Credentials => command.arg("--request-credentials")?,
            SampleDialogRequest::Notification => command.arg("--request-notification")?,
        }
    } else {
        BootstrapCommand::new(node_path)?.arg(client_path)?
    };
    let child = launch(&command, &bootstrap)?;
    if let Some((
        mailbox,
        input_mailbox,
        close_signal,
        file_dialog_mailbox,
        file_text,
        notifications,
    )) = mailboxes
    {
        crate::win32::run_ui_session(
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
            notifications,
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
