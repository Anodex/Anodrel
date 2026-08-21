//! End-to-end proof that the fixed native HTTPS diagnostic uses only its
//! authenticated invitation and public protocol result shape.

use std::{
    sync::{Arc, Mutex},
    thread,
};

use anodrel_core::{HostPolicy, HostServices};
use anodrel_network::{
    NetworkTextResponse, NetworkTextService, NetworkTextServiceError, NetworkUrl,
};
use anodrel_protocol::Capability;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const CHILD_TIMEOUT_MILLISECONDS: u32 = 10_000;

#[derive(Clone, Debug, Default)]
struct RecordingNetwork {
    requested_urls: Arc<Mutex<Vec<String>>>,
}

impl NetworkTextService for RecordingNetwork {
    fn fetch_text(&self, url: &NetworkUrl) -> Result<NetworkTextResponse, NetworkTextServiceError> {
        self.requested_urls
            .lock()
            .expect("network recorder lock is available")
            .push(url.as_str().to_owned());
        NetworkTextResponse::new(200, "diagnostic response")
            .map_err(|_| NetworkTextServiceError::ResponseInvalid)
    }
}

#[test]
fn the_compiled_probe_fetches_only_its_fixed_url_through_the_private_pipe() {
    let recorder = RecordingNetwork::default();
    let requested_urls = Arc::clone(&recorder.requested_urls);
    let policy = HostPolicy::new(
        "anodrel.native-network-client-sample",
        vec![Capability::NetworkFetch],
        "anodrel-native-network-client-sample-test-host",
    )
    .expect("test policy is valid");
    let (server, invitation) = WindowsPipeServer::create_with_services(
        policy,
        "native-network-client-sample-session",
        HostServices::unavailable().with_network(recorder),
    )
    .expect("test pipe server creates");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("test invitation converts");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(env!("CARGO_BIN_EXE_anodrel-native-network-client-sample"))
        .expect("probe executable path is valid");
    let child = launch(&command, &bootstrap).expect("compiled probe launches");

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("compiled probe exits within its bound"),
        0,
        "the native HTTPS diagnostic stopped at one of its safe stages"
    );
    worker
        .join()
        .expect("test pipe worker does not panic")
        .expect("test pipe worker completes");
    assert_eq!(
        *requested_urls
            .lock()
            .expect("network recorder lock is available"),
        vec!["https://example.com/"],
    );
}

#[test]
fn the_compiled_probe_refuses_to_search_without_bootstrap() {
    let mut child =
        std::process::Command::new(env!("CARGO_BIN_EXE_anodrel-native-network-client-sample"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("probe executable starts");
    let status = child.wait().expect("probe child exits");
    assert_eq!(status.code(), Some(51));
}
