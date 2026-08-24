//! End-to-end proof for the compiled two-document structure-event diagnostic.

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

const PREPARE_ACTION: &str = "native.structure.prepare";
const COMPLETE_ACTION: &str = "native.structure.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn the_compiled_child_completes_one_authenticated_document_replacement() {
    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let policy = HostPolicy::new(
        "anodrel.native-structure-event-client",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::SessionClose,
        ],
        "anodrel-native-structure-event-client-test-host",
    )
    .expect("test policy is valid");
    let (server, invitation) = WindowsPipeServer::create_with_session_components(
        policy,
        "native-structure-event-client-test-session",
        document_mailbox.clone(),
        input_mailbox.clone(),
        close_signal.clone(),
    )
    .expect("test pipe server creates");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("test invitation converts");
    let worker = thread::spawn(move || server.serve_one());
    let command =
        BootstrapCommand::new(env!("CARGO_BIN_EXE_anodrel-native-structure-event-client"))
            .expect("structure-event diagnostic executable path is valid");
    let child = launch(&command, &bootstrap).expect("compiled diagnostic launches");

    let initial = wait_for_document(&document_mailbox, &child, "initial");
    assert_eq!(initial.revision().value(), 1);
    input_mailbox.push(UiInputCandidate::new(
        initial.revision(),
        UiEvent::ActionInvoked(fixed_action(PREPARE_ACTION)),
    ));

    let replacement = wait_for_document(&document_mailbox, &child, "replacement");
    assert_eq!(replacement.revision().value(), 2);
    input_mailbox.push(UiInputCandidate::new(
        replacement.revision(),
        UiEvent::ActionInvoked(fixed_action(COMPLETE_ACTION)),
    ));

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("compiled diagnostic exits within its bound"),
        0,
        "the two-document diagnostic stopped at one of its safe stages"
    );
    assert!(
        close_signal.take(),
        "the diagnostic must request session.close after its second action"
    );
    worker
        .join()
        .expect("test pipe worker does not panic")
        .expect("test pipe worker completes");
}

#[test]
fn the_compiled_child_refuses_to_search_without_bootstrap() {
    let mut child =
        std::process::Command::new(env!("CARGO_BIN_EXE_anodrel-native-structure-event-client"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("compiled diagnostic starts");
    let status = child.wait().expect("compiled diagnostic exits");
    assert_eq!(status.code(), Some(61));
}

fn fixed_action(action: &str) -> ElementId {
    ElementId::new(action).expect("the compiled action ID is valid")
}

fn wait_for_document(
    mailbox: &UiDocumentMailbox,
    child: &LaunchedProcess,
    stage: &str,
) -> anodrel_ui_session::UiDocumentSnapshot {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(snapshot) = mailbox.take() {
            return snapshot;
        }
        if let Ok(exit_code) = child.wait_for_exit(0) {
            panic!(
                "the compiled diagnostic stopped at safe stage {exit_code} before its {stage} document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("the compiled diagnostic did not deliver its {stage} document within its bound");
}
