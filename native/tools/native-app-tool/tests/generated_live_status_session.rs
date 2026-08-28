//! End-to-end proof for a generated native live-status project.

#![forbid(unsafe_code)]

mod support;

use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use support::TestDirectory;

use anodrel_core::{HostPolicy, SessionCloseSignal};
use anodrel_protocol::Capability;
use anodrel_ui::{ElementId, UiEvent, UiStatusPoliteness};
use anodrel_ui_session::{UiDocumentMailbox, UiInputCandidate, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const POLITE_ACTION: &str = "template.status.polite";
const ASSERTIVE_ACTION: &str = "template.status.assertive";
const COMPLETE_ACTION: &str = "template.status.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn generated_project_replaces_only_explicit_v3_status_documents() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-live-status-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-live-status",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-live-status-session-app",
            "Generated Live Status Session App",
        ])
        .output()
        .expect("run native live-status application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid live-status input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native live-status project");
    assert!(
        built.success(),
        "generated native live-status project must build"
    );
    let executable = target_directory
        .join("release")
        .join("generated-live-status-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated live-status executable"
    );

    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let policy = HostPolicy::new(
        "anodrel.native-live-status-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed live-status template policy is valid");
    let (server, invitation) = WindowsPipeServer::create_with_session_components(
        policy,
        "native-live-status-template-session",
        document_mailbox.clone(),
        input_mailbox.clone(),
        close_signal.clone(),
    )
    .expect("create fixed live-status template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert live-status invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated live-status child");

    let initial = wait_for_document(&document_mailbox, &child);
    assert_status(
        &initial,
        1,
        "Ready to publish a visible result.",
        UiStatusPoliteness::Polite,
    );
    deliver_action(&input_mailbox, &initial, POLITE_ACTION);

    let polite = wait_for_document(&document_mailbox, &child);
    assert_status(
        &polite,
        2,
        "Verification is complete.",
        UiStatusPoliteness::Polite,
    );
    deliver_action(&input_mailbox, &polite, ASSERTIVE_ACTION);

    let assertive = wait_for_document(&document_mailbox, &child);
    assert_status(
        &assertive,
        3,
        "Verification succeeded.",
        UiStatusPoliteness::Assertive,
    );
    deliver_action(&input_mailbox, &assertive, COMPLETE_ACTION);

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated live-status template exits within its bound"),
        0,
        "generated live-status template completes each fixed stage"
    );
    assert!(
        close_signal.take(),
        "generated live-status template must close only its own session"
    );
    worker
        .join()
        .expect("live-status template pipe worker does not panic")
        .expect("live-status template pipe worker completes");
}

fn assert_status(
    snapshot: &anodrel_ui_session::UiDocumentSnapshot,
    expected_revision: u64,
    expected_value: &str,
    expected_politeness: UiStatusPoliteness,
) {
    assert_eq!(snapshot.revision().value(), expected_revision);
    let status = snapshot
        .document()
        .status()
        .expect("every generated v3 document has one status");
    assert_eq!(status.value(), expected_value);
    assert_eq!(status.politeness(), expected_politeness);
}

fn deliver_action(
    mailbox: &UiInputMailbox,
    snapshot: &anodrel_ui_session::UiDocumentSnapshot,
    action: &str,
) {
    mailbox.push(UiInputCandidate::new(
        snapshot.revision(),
        UiEvent::ActionInvoked(ElementId::new(action).expect("fixed action ID is valid")),
    ));
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
        assert_child_is_running(child, "delivering its live-status document");
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated live-status template did not deliver a document within its bound");
}

fn assert_child_is_running(child: &LaunchedProcess, stage: &str) {
    if let Ok(exit_code) = child.wait_for_exit(0) {
        panic!("generated live-status template stopped at safe stage {exit_code} before {stage}");
    }
}
