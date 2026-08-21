//! Host orchestration for the compiled native UI-session diagnostic.
//!
//! This development-only route carries one explicit native executable through
//! the existing private bootstrap, authenticated pipe, and host-owned UI
//! session. It deliberately grants only document replacement, event pull, and
//! closing that one session; it is not a product launcher or application API.

use std::{error::Error, io, thread};

use anodrel_core::HostPolicy;
use anodrel_protocol::Capability;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_pipe::WindowsPipeServer;

use crate::session_ui::DevelopmentSessionUi;

const CHILD_TIMEOUT_MILLISECONDS: u32 = 10_000;
const DEVELOPMENT_STOP_CODE: u32 = 0xA11D;
const DISPLAY_NAME: &str = "Anodrel Native UI Probe";

/// Runs the selected compiled child through one real native UI session.
///
/// The child is unverified by design: a developer names it explicitly, its
/// output is discarded by the bootstrap adapter, and it receives only this
/// session's one-time invitation. A signed product session is a separate path.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    // Validate the selected command before a worker is allowed to wait for a
    // process that cannot start.
    let command = BootstrapCommand::new(client_path)?;
    let ui = DevelopmentSessionUi::new();
    let policy = HostPolicy::new(
        "anodrel.native-ui-client-sample",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::SessionClose,
        ],
        "anodrel-windows-host",
    )?;
    let (server, invitation) = WindowsPipeServer::create_with_session_components(
        policy,
        "native-ui-client-sample-session",
        ui.document.clone(),
        ui.input.clone(),
        ui.close.clone(),
    )?;
    let stop = server.stop_signal();
    let bootstrap = invitation.bootstrap_invitation()?;
    let worker = thread::spawn(move || server.serve_one());
    let child = match launch(&command, &bootstrap) {
        Ok(child) => child,
        Err(error) => {
            stop.request_stop();
            let _ = worker.join();
            return Err(error.into());
        }
    };

    if let Err(error) = crate::win32::run_ui_session(
        ui.document,
        ui.input,
        ui.close,
        ui.file_dialog,
        ui.file_text,
        ui.notifications,
        ui.menu,
        ui.window_title,
        ui.window_state,
        DISPLAY_NAME,
        ui.fields,
    ) {
        let _ = child.terminate(DEVELOPMENT_STOP_CODE);
        stop.request_stop();
        let _ = worker.join();
        return Err(error.into());
    }

    let exit_code = match child.wait_for_exit(CHILD_TIMEOUT_MILLISECONDS) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            let _ = child.terminate(DEVELOPMENT_STOP_CODE);
            stop.request_stop();
            let _ = worker.join();
            return Err(error.into());
        }
    };
    if exit_code != 0 {
        stop.request_stop();
        let _ = worker.join();
        return Err(io::Error::other(format!(
            "compiled native UI development probe failed at safe stage {exit_code}"
        ))
        .into());
    }
    worker
        .join()
        .map_err(|_| io::Error::other("native UI probe pipe worker panicked"))??;
    println!("Anodrel native UI development probe completed successfully.");
    Ok(())
}
