//! Host orchestration for the fixed-origin compiled HTTPS development probe.
//!
//! This intentionally has no relationship to the normal native template or
//! product-session paths. It grants one operator-selected diagnostic executable
//! the one `network.fetch` capability against one origin compiled by the host.

use std::{error::Error, io, thread};

use anodrel_core::{HostPolicy, HostServices};
use anodrel_network::{NetworkOrigin, NetworkOriginPolicy};
use anodrel_protocol::Capability;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_network::WindowsNetworkTextService;
use anodrel_windows_pipe::WindowsPipeServer;

const SAMPLE_TIMEOUT_MILLISECONDS: u32 = 45_000;
const DEVELOPMENT_STOP_CODE: u32 = 0xA11D;
const HOST_NAME: &str = "anodrel-windows-host";
const DIAGNOSTIC_APPLICATION_ID: &str = "anodrel.native-network-client-sample";
const DIAGNOSTIC_SESSION_ID: &str = "native-network-client-sample-session";
const DIAGNOSTIC_ORIGIN_HOST: &str = "example.com";
const DIAGNOSTIC_ORIGIN_PORT: u16 = 443;
const NETWORK_GRANTS: [Capability; 1] = [Capability::NetworkFetch];

/// Runs one explicitly selected diagnostic through the private bootstrap and
/// authenticated fixed-origin HTTPS text-fetch path.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    // Validate the operator-selected command before creating a worker that
    // could otherwise wait for a child that can never be launched.
    let command = BootstrapCommand::new(client_path)?;
    let policy = HostPolicy::new(
        DIAGNOSTIC_APPLICATION_ID,
        NETWORK_GRANTS.to_vec(),
        HOST_NAME,
    )?;
    let origins = NetworkOriginPolicy::new(vec![NetworkOrigin::new(
        DIAGNOSTIC_ORIGIN_HOST,
        DIAGNOSTIC_ORIGIN_PORT,
    )?])?;
    let services =
        HostServices::unavailable().with_network(WindowsNetworkTextService::new(origins));
    let (server, invitation) =
        WindowsPipeServer::create_with_services(policy, DIAGNOSTIC_SESSION_ID, services)?;
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
        stop.request_stop();
        let _ = worker.join();
        return Err(io::Error::other(format!(
            "compiled native HTTPS development diagnostic failed at safe stage {exit_code}"
        ))
        .into());
    }
    worker
        .join()
        .map_err(|_| io::Error::other("native HTTPS diagnostic pipe worker panicked"))??;
    println!("Anodrel native HTTPS development diagnostic completed successfully.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use anodrel_network::{NetworkOrigin, NetworkOriginPolicy, NetworkUrl};
    use anodrel_protocol::Capability;

    use super::{DIAGNOSTIC_ORIGIN_HOST, DIAGNOSTIC_ORIGIN_PORT, NETWORK_GRANTS};

    #[test]
    fn diagnostic_policy_has_one_compiled_origin_and_one_capability() {
        let origins = NetworkOriginPolicy::new(vec![
            NetworkOrigin::new(DIAGNOSTIC_ORIGIN_HOST, DIAGNOSTIC_ORIGIN_PORT)
                .expect("compiled origin is valid"),
        ])
        .expect("compiled origin policy is valid");
        assert_eq!(NETWORK_GRANTS, [Capability::NetworkFetch]);
        assert!(
            origins.allows(
                &NetworkUrl::parse("https://example.com/").expect("diagnostic URL is valid")
            )
        );
        assert!(
            !origins.allows(
                &NetworkUrl::parse("https://www.example.com/").expect("foreign URL is valid")
            )
        );
    }
}
