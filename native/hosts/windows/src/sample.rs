//! Development-only end-to-end sample orchestration.
//!
//! This path proves the private bootstrap and named-pipe contract. It does not
//! verify the child executable or provide product application lifecycle policy.

use std::{error::Error, io, thread};

use anodrel_core::{HostPolicy, SessionCloseSignal};
use anodrel_file_dialog::FileDialogMailbox;
use anodrel_protocol::Capability;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_clipboard::WindowsClipboard;
use anodrel_windows_external_links::WindowsExternalLinks;
use anodrel_windows_pipe::WindowsPipeServer;

const SAMPLE_TIMEOUT_MILLISECONDS: u32 = 10_000;

pub fn run(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_with_optional_session_view(node_path, client_path, None, false)
}

/// Runs the development bootstrap sample while one native window consumes its
/// authenticated document mailbox.
pub fn run_ui_session(node_path: &str, client_path: &str) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_optional_file_dialog(node_path, client_path, false)
}

/// Runs the UI session diagnostic and asks its client to show one file picker.
pub fn run_ui_session_with_file_dialog(
    node_path: &str,
    client_path: &str,
) -> Result<(), Box<dyn Error>> {
    run_ui_session_with_optional_file_dialog(node_path, client_path, true)
}

fn run_ui_session_with_optional_file_dialog(
    node_path: &str,
    client_path: &str,
    request_file_dialog: bool,
) -> Result<(), Box<dyn Error>> {
    let mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let file_dialog_mailbox = FileDialogMailbox::new();
    run_with_optional_session_view(
        node_path,
        client_path,
        Some((mailbox, input_mailbox, close_signal, file_dialog_mailbox)),
        request_file_dialog,
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
    )>,
    request_file_dialog: bool,
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
        ],
        "anodrel-windows-host",
    )?;
    let (server, invitation) = match mailboxes.as_ref() {
        Some((mailbox, input_mailbox, close_signal, file_dialog_mailbox)) => {
            WindowsPipeServer::create_with_session_components_and_all_services(
                policy,
                "sample-session",
                mailbox.clone(),
                input_mailbox.clone(),
                close_signal.clone(),
                WindowsClipboard::new(0),
                WindowsExternalLinks,
                file_dialog_mailbox.clone(),
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
        if request_file_dialog {
            command.arg("--request-open-file")?
        } else {
            command
        }
    } else {
        BootstrapCommand::new(node_path)?.arg(client_path)?
    };
    let child = launch(&command, &bootstrap)?;
    if let Some((mailbox, input_mailbox, close_signal, file_dialog_mailbox)) = mailboxes {
        crate::win32::run_ui_session(mailbox, input_mailbox, close_signal, file_dialog_mailbox)?;
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
