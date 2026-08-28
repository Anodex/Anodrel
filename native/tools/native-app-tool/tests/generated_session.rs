//! End-to-end proof for an executable project created by the native generator.

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
use anodrel_ui::{ElementId, UiEvent};
use anodrel_ui_session::{UiDocumentMailbox, UiInputCandidate, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const TEMPLATE_ACTION: &str = "template.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn generated_project_completes_one_fixed_authenticated_ui_session() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-session-app",
            "Generated Session App",
        ])
        .output()
        .expect("run native application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native project");
    assert!(built.success(), "generated project must build");
    let executable = target_directory
        .join("release")
        .join("generated-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated executable"
    );

    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let policy = HostPolicy::new(
        "anodrel.native-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed template policy is valid");
    let (server, invitation) = WindowsPipeServer::create_with_session_components(
        policy,
        "native-template-session",
        document_mailbox.clone(),
        input_mailbox.clone(),
        close_signal.clone(),
    )
    .expect("create fixed template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert template invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated template child");

    let snapshot = wait_for_document(&document_mailbox, &child);
    assert_eq!(snapshot.revision().value(), 1);
    input_mailbox.push(UiInputCandidate::new(
        snapshot.revision(),
        UiEvent::ActionInvoked(
            ElementId::new(TEMPLATE_ACTION).expect("fixed template action is valid"),
        ),
    ));

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated template exits within its bound"),
        0,
        "generated template must complete rather than stop at a safe stage"
    );
    assert!(
        close_signal.take(),
        "generated template must request close only for its own session"
    );
    worker
        .join()
        .expect("template pipe worker does not panic")
        .expect("template pipe worker completes");
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
                "generated template stopped at safe stage {exit_code} before delivering its document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated template did not deliver its document within its bound");
}
