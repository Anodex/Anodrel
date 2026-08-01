//! Development-only end-to-end sample orchestration.
//!
//! This path proves the private bootstrap and named-pipe contract. It does not
//! verify the child executable or provide product application lifecycle policy.

use std::{error::Error, io, thread};

use anodrel_core::{HostPolicy, SessionCloseSignal};
use anodrel_file_access::SelectionFileDialogMailbox;
use anodrel_file_dialog::FileDialogMailbox;
use anodrel_protocol::Capability;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_clipboard::WindowsClipboard;
use anodrel_windows_external_links::WindowsExternalLinks;
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_pipe::WindowsPipeServer;

const SAMPLE_TIMEOUT_MILLISECONDS: u32 = 10_000;

#[derive(Clone, Copy)]
enum SampleDialogRequest {
    None,
    OpenFile,
    OpenFileWithReference,
    SaveFile,
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
    run_with_optional_session_view(
        node_path,
        client_path,
        Some((
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
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
        ],
        "anodrel-windows-host",
    )?;
    let (server, invitation) = match mailboxes.as_ref() {
        Some((mailbox, input_mailbox, close_signal, file_dialog_mailbox, file_text)) => {
            WindowsPipeServer::create_with_session_components_and_all_services_and_file_access(
                policy,
                "sample-session",
                mailbox.clone(),
                input_mailbox.clone(),
                close_signal.clone(),
                WindowsClipboard::new(0),
                WindowsExternalLinks,
                file_dialog_mailbox.clone(),
                SelectionFileDialogMailbox::new(file_dialog_mailbox.clone()),
                file_text.clone(),
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
        }
    } else {
        BootstrapCommand::new(node_path)?.arg(client_path)?
    };
    let child = launch(&command, &bootstrap)?;
    if let Some((mailbox, input_mailbox, close_signal, file_dialog_mailbox, file_text)) = mailboxes
    {
        crate::win32::run_ui_session(
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
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
