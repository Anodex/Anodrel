//! Host orchestration for the compiled native child development probe.
//!
//! This is the no-runtime counterpart to the existing Node-based diagnostic.
//! It is deliberately narrow: the caller selects an executable, the host gives
//! it one private invitation, and the child proves `platform.health` before it
//! exits. No executable identity, package, capability bridge, or product
//! lifetime policy is implied.

use std::{error::Error, io, thread};

use anodrel_core::HostPolicy;
use anodrel_protocol::Capability;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const SAMPLE_TIMEOUT_MILLISECONDS: u32 = 10_000;
const DEVELOPMENT_STOP_CODE: u32 = 0xA11D;

/// Runs one explicitly selected compiled native child through the private
/// bootstrap, authentication, and health path.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    // Validate the operator-selected command before creating a worker that
    // could otherwise wait for a client that can never be launched.
    let command = BootstrapCommand::new(client_path)?;
    let policy = HostPolicy::new(
        "anodrel.native-client-sample",
        vec![Capability::DiagnosticsRead],
        "anodrel-windows-host",
    )?;
    let (server, invitation) = WindowsPipeServer::create(policy, "native-client-sample-session")?;
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

    let exit_code = match child.wait_for_exit(SAMPLE_TIMEOUT_MILLISECONDS) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            let _ = child.terminate(DEVELOPMENT_STOP_CODE);
            stop.request_stop();
            let _ = worker.join();
            return Err(error.into());
        }
    };
    if exit_code != 0 {
        let _ = worker.join();
        return Err(io::Error::other(format!(
            "compiled native development probe failed at safe stage {exit_code}"
        ))
        .into());
    }
    worker
        .join()
        .map_err(|_| io::Error::other("native probe pipe worker panicked"))??;
    println!("Anodrel native development probe completed successfully.");
    Ok(())
}
