use std::time::{SystemTime, UNIX_EPOCH};

use anodrel_core::HostPolicy;
use anodrel_protocol::Capability;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const SESSION_ID: &str = "integration-session";

#[test]
fn delivers_one_private_bootstrap_frame_to_a_real_child_process() {
    let result_path = std::env::temp_dir().join(format!(
        "anodrel-bootstrap-{}-{}.txt",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is after epoch")
            .as_nanos()
    ));
    let fixture = env!("CARGO_BIN_EXE_anodrel-bootstrap-fixture");
    let command = BootstrapCommand::new(fixture)
        .expect("fixture path is valid")
        .arg(result_path.to_string_lossy())
        .expect("result path is valid");
    let policy = HostPolicy::new(
        "integration.application",
        vec![Capability::DiagnosticsRead],
        "integration-host",
    )
    .expect("integration policy is valid");
    let (_server, invitation) =
        WindowsPipeServer::create(policy, SESSION_ID).expect("named pipe creates");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("pipe invitation converts to bootstrap");
    let child = launch(&command, &bootstrap).expect("child launches and receives bootstrap");
    assert_eq!(child.wait_for_exit(5_000).expect("child exits"), 0);
    let result = std::fs::read_to_string(&result_path).expect("fixture wrote only safe fields");
    std::fs::remove_file(&result_path).expect("fixture output is cleaned up");
    assert_eq!(
        result,
        format!("{}\n{SESSION_ID}\n", invitation.pipe_name())
    );
    assert!(!result.contains("token"));
}

#[test]
fn tracked_child_can_be_terminated_during_host_shutdown() {
    let result_path = std::env::temp_dir().join(format!(
        "anodrel-bootstrap-shutdown-{}-{}.txt",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is after epoch")
            .as_nanos()
    ));
    let fixture = env!("CARGO_BIN_EXE_anodrel-bootstrap-fixture");
    let command = BootstrapCommand::new(fixture)
        .expect("fixture path is valid")
        .arg(result_path.to_string_lossy())
        .expect("result path is valid")
        .arg("--wait")
        .expect("wait argument is valid");
    let policy = HostPolicy::new(
        "integration.shutdown",
        vec![Capability::DiagnosticsRead],
        "integration-host",
    )
    .expect("test policy is valid");
    let (_server, invitation) =
        WindowsPipeServer::create(policy, "shutdown-session").expect("named pipe creates");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("pipe invitation converts to bootstrap");
    let child = launch(&command, &bootstrap).expect("child launches and receives bootstrap");

    for _ in 0..100 {
        if result_path.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        result_path.exists(),
        "fixture received its private invitation"
    );

    child.terminate(0xA11D).expect("host terminates child");
    assert_eq!(child.wait_for_exit(5_000).expect("child exits"), 0xA11D);
    std::fs::remove_file(&result_path).expect("fixture output is cleaned up");
}
