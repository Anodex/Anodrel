//! End-to-end proof for an executable project created by the notification generator.

#![forbid(unsafe_code)]

mod support;

use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use support::TestDirectory;

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_notifications::NotificationMailbox;
use anodrel_protocol::Capability;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const NOTIFICATION_TITLE: &str = "Anodrel native notification";
const NOTIFICATION_BODY: &str =
    "The direct Windows notification route accepted this fixed template request.";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 10_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn generated_notification_project_completes_one_fixed_authenticated_session() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-notification-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-notification",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-notification-session-app",
            "Generated Notification Session App",
        ])
        .output()
        .expect("run native notification application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid notification input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native notification project");
    assert!(
        built.success(),
        "generated native notification project must build"
    );
    let executable = target_directory
        .join("release")
        .join("generated-notification-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated notification executable"
    );

    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let notification_mailbox = NotificationMailbox::new();
    let policy = HostPolicy::new(
        "anodrel.native-notification-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::NotificationShow,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed notification template policy is valid");
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            "native-notification-template-session",
            document_mailbox.clone(),
            input_mailbox,
            close_signal.clone(),
            HostServices::unavailable().with_notifications(notification_mailbox.clone()),
        )
        .expect("create fixed notification template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert notification template invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated notification template child");

    let snapshot = wait_for_document(&document_mailbox, &child);
    assert_eq!(snapshot.revision().value(), 1);
    let request = wait_for_notification(&notification_mailbox, &child);
    assert_eq!(request.notification().title().as_str(), NOTIFICATION_TITLE);
    assert_eq!(request.notification().body().as_str(), NOTIFICATION_BODY);
    assert!(notification_mailbox.complete(request.id()));

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated notification template exits within its bound"),
        0,
        "generated notification template must complete rather than stop at a safe stage"
    );
    assert!(
        close_signal.take(),
        "generated notification template must request close only for its own session"
    );
    worker
        .join()
        .expect("notification-template pipe worker does not panic")
        .expect("notification-template pipe worker completes");
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
                "generated notification template stopped at safe stage {exit_code} before delivering its document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated notification template did not deliver its document within its bound");
}

fn wait_for_notification(
    mailbox: &NotificationMailbox,
    child: &LaunchedProcess,
) -> anodrel_notifications::NotificationRequest {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(request) = mailbox.take() {
            return request;
        }
        if let Ok(exit_code) = child.wait_for_exit(0) {
            panic!(
                "generated notification template stopped at safe stage {exit_code} before requesting its notification"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated notification template did not request its notification within its bound");
}
