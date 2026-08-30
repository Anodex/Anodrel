//! End-to-end proof for an executable project created by the context-menu generator.

#![forbid(unsafe_code)]

mod support;

use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use support::TestDirectory;

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_menu::{ContextMenuMailbox, MenuActionId};
use anodrel_protocol::Capability;
use anodrel_ui_session::{ContextMenuInputCandidate, UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const CONTEXT_ACTION: &str = "template.context.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn generated_context_menu_project_completes_one_fixed_authenticated_session() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-context-menu-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-context-menu",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-context-menu-session-app",
            "Generated Context Menu Session App",
        ])
        .output()
        .expect("run native context-menu application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid context-menu input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native context-menu project");
    assert!(
        built.success(),
        "generated native context-menu project must build"
    );
    let executable = target_directory
        .join("release")
        .join("generated-context-menu-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated context-menu executable"
    );

    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let context_menu_mailbox = ContextMenuMailbox::new();
    let policy = HostPolicy::new(
        "anodrel.native-context-menu-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::ContextMenuWrite,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed context-menu template policy is valid");
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            "native-context-menu-template-session",
            document_mailbox.clone(),
            input_mailbox.clone(),
            close_signal.clone(),
            HostServices::unavailable().with_context_menu(context_menu_mailbox.clone()),
        )
        .expect("create fixed context-menu template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert context-menu template invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated context-menu template child");

    let snapshot = wait_for_document(&document_mailbox, &child);
    assert_eq!(snapshot.revision().value(), 1);
    let request = wait_for_context_menu(&context_menu_mailbox, &child);
    assert_eq!(request.revision().value(), 1);
    assert_eq!(request.model().items().len(), 1);
    assert_eq!(request.model().items()[0].id().as_str(), CONTEXT_ACTION);
    assert!(request.model().items()[0].enabled());
    assert!(request.model().items()[0].shortcut().is_none());
    assert!(context_menu_mailbox.complete(request.id()));
    input_mailbox.push(ContextMenuInputCandidate::new(
        request.revision(),
        MenuActionId::new(CONTEXT_ACTION).expect("fixed context action is valid"),
    ));

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated context-menu template exits within its bound"),
        0,
        "generated context-menu template must complete rather than stop at a safe stage"
    );
    assert!(
        close_signal.take(),
        "generated context-menu template must request close only for its own session"
    );
    worker
        .join()
        .expect("context-menu template pipe worker does not panic")
        .expect("context-menu template pipe worker completes");
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
                "generated context-menu template stopped at safe stage {exit_code} before delivering its document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated context-menu template did not deliver its document within its bound");
}

fn wait_for_context_menu(
    mailbox: &ContextMenuMailbox,
    child: &LaunchedProcess,
) -> anodrel_menu::ContextMenuRequest {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(request) = mailbox.take() {
            return request;
        }
        if let Ok(exit_code) = child.wait_for_exit(0) {
            panic!(
                "generated context-menu template stopped at safe stage {exit_code} before requesting its context menu"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated context-menu template did not request its context menu within its bound");
}
