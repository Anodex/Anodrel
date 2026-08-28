//! End-to-end proof for an executable project created by the native menu generator.

#![forbid(unsafe_code)]

mod support;

use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use support::TestDirectory;

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_menu::{MenuActionId, MenuMailbox};
use anodrel_protocol::Capability;
use anodrel_ui_session::{MenuInputCandidate, UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const MENU_ACTION: &str = "template.menu.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn generated_menu_project_completes_one_fixed_authenticated_menu_session() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-menu-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-menu",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-menu-session-app",
            "Generated Menu Session App",
        ])
        .output()
        .expect("run native menu application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid menu input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native menu project");
    assert!(built.success(), "generated native menu project must build");
    let executable = target_directory
        .join("release")
        .join("generated-menu-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated menu executable"
    );

    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let menu_mailbox = MenuMailbox::new();
    let policy = HostPolicy::new(
        "anodrel.native-menu-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::MenuWrite,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed menu-template policy is valid");
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            "native-menu-template-session",
            document_mailbox.clone(),
            input_mailbox.clone(),
            close_signal.clone(),
            HostServices::unavailable().with_menu(menu_mailbox.clone()),
        )
        .expect("create fixed menu-template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert menu-template invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated menu template child");

    let snapshot = wait_for_document(&document_mailbox, &child);
    assert_eq!(snapshot.revision().value(), 1);
    let request = wait_for_menu(&menu_mailbox, &child);
    assert_eq!(request.revision().value(), 1);
    assert_eq!(request.model().menus().len(), 1);
    assert_eq!(
        request.model().menus()[0].items()[0].id().as_str(),
        MENU_ACTION
    );
    assert_eq!(
        request.model().menus()[0].items()[0]
            .shortcut()
            .expect("generated menu action declares its fixed local shortcut")
            .display_text(),
        "Ctrl+Shift+M"
    );
    assert!(menu_mailbox.complete(request.id()));
    input_mailbox.push(MenuInputCandidate::new(
        request.revision(),
        MenuActionId::new(MENU_ACTION).expect("fixed menu action is valid"),
    ));

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated menu template exits within its bound"),
        0,
        "generated menu template must complete rather than stop at a safe stage"
    );
    assert!(
        close_signal.take(),
        "generated menu template must request close only for its own session"
    );
    worker
        .join()
        .expect("menu-template pipe worker does not panic")
        .expect("menu-template pipe worker completes");
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
                "generated menu template stopped at safe stage {exit_code} before delivering its document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated menu template did not deliver its document within its bound");
}

fn wait_for_menu(mailbox: &MenuMailbox, child: &LaunchedProcess) -> anodrel_menu::MenuRequest {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(request) = mailbox.take() {
            return request;
        }
        if let Ok(exit_code) = child.wait_for_exit(0) {
            panic!(
                "generated menu template stopped at safe stage {exit_code} before requesting its menu"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated menu template did not request its menu within its bound");
}
