#![cfg(target_os = "linux")]

use std::{
    io::Write,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use anodrel_core::HostPolicy;
use anodrel_linux_pipe::LinuxPipeServer;
use anodrel_protocol::Capability;

const CHILD_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_INTERVAL: Duration = Duration::from_millis(10);

#[test]
fn compiled_probe_completes_one_invited_linux_health_round_trip() {
    let policy = HostPolicy::new(
        "anodrel.native-linux-client-sample",
        vec![Capability::DiagnosticsRead],
        "anodrel-native-linux-client-sample-test-host",
    )
    .expect("test policy is valid");
    let (server, invitation) =
        LinuxPipeServer::create(policy, "native-linux-client-sample-session")
            .expect("Linux server creates");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("transport invitation converts");
    let encoded = bootstrap.encode().expect("bootstrap encodes");
    let worker = thread::spawn(move || server.serve_one());

    let mut child = Command::new(env!("CARGO_BIN_EXE_anodrel-native-linux-client-sample"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("probe starts");
    child
        .stdin
        .take()
        .expect("probe receives private standard input")
        .write_all(&encoded)
        .expect("bootstrap writes");

    assert_eq!(
        wait_for_exit(&mut child).code(),
        Some(0),
        "the Linux probe stopped at one of its safe stages"
    );
    worker
        .join()
        .expect("Linux worker does not panic")
        .expect("Linux worker completes");
}

#[test]
fn compiled_probe_refuses_to_search_without_a_linux_invitation() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_anodrel-native-linux-client-sample"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("probe starts");
    assert_eq!(wait_for_exit(&mut child).code(), Some(31));
}

fn wait_for_exit(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + CHILD_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().expect("probe status is readable") {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("Linux probe did not exit inside its bound");
        }
        thread::sleep(WAIT_INTERVAL);
    }
}
