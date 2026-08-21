//! End-to-end proof that the compiled development probe needs no Node runtime.

use std::thread;

use anodrel_core::HostPolicy;
use anodrel_protocol::Capability;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const CHILD_TIMEOUT_MILLISECONDS: u32 = 10_000;

#[test]
fn the_compiled_probe_completes_the_private_health_round_trip() {
    let policy = HostPolicy::new(
        "anodrel.native-client-sample",
        vec![Capability::DiagnosticsRead],
        "anodrel-native-client-sample-test-host",
    )
    .expect("test policy is valid");
    let (server, invitation) = WindowsPipeServer::create(policy, "native-client-sample-session")
        .expect("test pipe server creates");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("test invitation converts");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(env!("CARGO_BIN_EXE_anodrel-native-client-sample"))
        .expect("probe executable path is valid");
    let child = launch(&command, &bootstrap).expect("compiled probe launches");

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("compiled probe exits within its bound"),
        0,
        "the native probe stopped at one of its safe stages"
    );
    worker
        .join()
        .expect("test pipe worker does not panic")
        .expect("test pipe worker completes");
}

#[test]
fn the_compiled_probe_refuses_to_search_without_bootstrap() {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_anodrel-native-client-sample"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("probe executable starts");
    let status = child.wait().expect("probe child exits");
    assert_eq!(status.code(), Some(31));
}
