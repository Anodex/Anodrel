#![cfg(target_os = "linux")]

use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

use anodrel_core::HostPolicy;
use anodrel_linux_bootstrap::LinuxBootstrapProgram;
use anodrel_linux_development_session::start_development_session;
use anodrel_protocol::Capability;

const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[test]
fn coordinator_owns_one_compiled_child_and_private_transport_until_completion() {
    let policy = HostPolicy::new(
        "anodrel.native-linux-development-session",
        vec![Capability::DiagnosticsRead],
        "anodrel-linux-development-session-test-host",
    )
    .expect("test policy is valid");
    let program = LinuxBootstrapProgram::new(Path::new(env!(
        "CARGO_BIN_EXE_anodrel-native-linux-client-sample"
    )))
    .expect("compiled client path is host-selected and absolute");
    let session = start_development_session(policy, "linux-development-session", program)
        .expect("host-owned development session starts");
    let close_signal = session.close_signal();

    let deadline = Instant::now() + COMPLETION_TIMEOUT;
    while !close_signal.take() {
        if Instant::now() >= deadline {
            drop(session);
            panic!("development session did not report its closed lifetime");
        }
        thread::sleep(POLL_INTERVAL);
    }

    session
        .finish()
        .expect("child and authenticated worker stop together");
}
