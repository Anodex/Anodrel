#![cfg(target_os = "linux")]

use std::{path::Path, thread, time::Duration};

use anodrel_core::HostPolicy;
use anodrel_linux_bootstrap::LinuxBootstrapProgram;
use anodrel_linux_development_session::start_development_session;
use anodrel_protocol::Capability;

const SETTLE_DURATION: Duration = Duration::from_millis(150);

#[test]
fn held_compiled_child_stays_host_owned_until_the_session_finishes() {
    let policy = HostPolicy::new(
        "anodrel.native-linux-session-window-lab",
        vec![Capability::DiagnosticsRead],
        "anodrel-linux-session-window-lab-test-host",
    )
    .expect("test policy is valid");
    let program = LinuxBootstrapProgram::new(Path::new(env!(
        "CARGO_BIN_EXE_anodrel-native-linux-session-client"
    )))
    .expect("compiled held client path is host-selected and absolute");
    let session = start_development_session(policy, "linux-held-session-client", program)
        .expect("held development session starts");
    let close_signal = session.close_signal();

    thread::sleep(SETTLE_DURATION);
    assert!(
        !close_signal.take(),
        "the compiled held child must not end its host-owned session early"
    );

    session
        .finish()
        .expect("session finish stops and joins the held child");
}
