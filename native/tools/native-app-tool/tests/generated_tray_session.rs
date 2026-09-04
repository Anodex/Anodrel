//! End-to-end proof for an executable project created by the tray generator.

#![forbid(unsafe_code)]

mod support;

use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use support::TestDirectory;

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_menu::{MenuActionId, TrayMailbox};
use anodrel_protocol::Capability;
use anodrel_ui_session::{TrayInputCandidate, UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const TRAY_ACTION: &str = "template.tray.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn generated_tray_project_completes_one_fixed_authenticated_session() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-tray-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-tray",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-tray-session-app",
            "Generated Tray Session App",
        ])
        .output()
        .expect("run native tray application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid tray input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native tray project");
    assert!(built.success(), "generated native tray project must build");
    let executable = target_directory
        .join("release")
        .join("generated-tray-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated tray executable"
    );

    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let tray_mailbox = TrayMailbox::new();
    let policy = HostPolicy::new(
        "anodrel.native-tray-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::TrayWrite,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed tray template policy is valid");
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            "native-tray-template-session",
            document_mailbox.clone(),
            input_mailbox.clone(),
            close_signal.clone(),
            HostServices::unavailable().with_tray(tray_mailbox.clone()),
        )
        .expect("create fixed tray template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert tray template invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated tray template child");

    let snapshot = wait_for_document(&document_mailbox, &child);
    assert_eq!(snapshot.revision().value(), 1);
    let request = wait_for_tray(&tray_mailbox, &child);
    assert_eq!(request.revision().value(), 1);
    assert_eq!(request.model().items().len(), 1);
    assert_eq!(request.model().items()[0].id().as_str(), TRAY_ACTION);
    assert!(request.model().items()[0].enabled());
    assert!(tray_mailbox.complete(request.id()));
    input_mailbox.push(TrayInputCandidate::new(
        request.revision(),
        MenuActionId::new(TRAY_ACTION).expect("fixed tray action is valid"),
    ));

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated tray template exits within its bound"),
        0,
        "generated tray template must complete rather than stop at a safe stage"
    );
    assert!(
        close_signal.take(),
        "generated tray template must request close only for its own session"
    );
    worker
        .join()
        .expect("tray-template pipe worker does not panic")
        .expect("tray-template pipe worker completes");
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
                "generated tray template stopped at safe stage {exit_code} before delivering its document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated tray template did not deliver its document within its bound");
}

fn wait_for_tray(mailbox: &TrayMailbox, child: &LaunchedProcess) -> anodrel_menu::TrayRequest {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(request) = mailbox.take() {
            return request;
        }
        if let Ok(exit_code) = child.wait_for_exit(0) {
            panic!(
                "generated tray template stopped at safe stage {exit_code} before requesting its tray"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated tray template did not request its tray within its bound");
}
