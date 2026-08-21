//! End-to-end proof for the compiled native interactive diagnostic.

use std::{
    thread,
    time::{Duration, Instant},
};

use anodrel_core::{HostPolicy, SessionCloseSignal};
use anodrel_protocol::Capability;
use anodrel_ui::{ElementId, UiEvent};
use anodrel_ui_session::{UiDocumentMailbox, UiInputCandidate, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const NATIVE_UI_ACTION: &str = "native.ui.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn the_compiled_native_ui_probe_completes_one_window_round_trip() {
    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let policy = HostPolicy::new(
        "anodrel.native-ui-client-sample",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::SessionClose,
        ],
        "anodrel-native-ui-client-sample-test-host",
    )
    .expect("test policy is valid");
    let (server, invitation) = WindowsPipeServer::create_with_session_components(
        policy,
        "native-ui-client-sample-test-session",
        document_mailbox.clone(),
        input_mailbox.clone(),
        close_signal.clone(),
    )
    .expect("test pipe server creates");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("test invitation converts");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(env!("CARGO_BIN_EXE_anodrel-native-ui-client-sample"))
        .expect("native UI probe executable path is valid");
    let child = launch(&command, &bootstrap).expect("compiled native UI probe launches");

    let snapshot = wait_for_document(&document_mailbox, &child);
    assert_eq!(
        snapshot.revision().value(),
        1,
        "a freshly created session must accept the native diagnostic document as revision 1"
    );
    input_mailbox.push(UiInputCandidate::new(
        snapshot.revision(),
        UiEvent::ActionInvoked(
            ElementId::new(NATIVE_UI_ACTION).expect("the fixed action ID is valid"),
        ),
    ));

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("compiled native UI probe exits within its bound"),
        0,
        "the native UI probe stopped at one of its safe stages"
    );
    assert!(
        close_signal.take(),
        "the native UI probe must request session.close before it exits"
    );
    worker
        .join()
        .expect("test pipe worker does not panic")
        .expect("test pipe worker completes");
}

#[test]
fn the_compiled_native_ui_probe_refuses_to_search_without_bootstrap() {
    let mut child =
        std::process::Command::new(env!("CARGO_BIN_EXE_anodrel-native-ui-client-sample"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("native UI probe executable starts");
    let status = child.wait().expect("native UI probe exits");
    assert_eq!(status.code(), Some(41));
}

fn wait_for_document(
    mailbox: &UiDocumentMailbox,
    child: &LaunchedProcess,
) -> anodrel_ui_session::UiDocumentSnapshot {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(snapshot) = mailbox.take() {
            return snapshot;
        }
        if let Ok(exit_code) = child.wait_for_exit(0) {
            panic!(
                "the native UI probe stopped at safe stage {exit_code} before delivering its document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("the native UI probe did not deliver its document within its bound");
}
