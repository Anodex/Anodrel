//! End-to-end proof for a project created by the native multi-window generator.

#![forbid(unsafe_code)]

mod support;

use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use support::TestDirectory;

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_protocol::Capability;
use anodrel_ui::{ElementId, UiEvent};
use anodrel_ui_session::{
    UiDocumentMailbox, UiInputCandidate, UiInputMailbox, UiWindowGroup, UiWindowId,
};
use anodrel_window::WindowTitleProposal;
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const PRIMARY_OPEN_ACTION: &str = "template.window.open";
const SECONDARY_UPDATE_ACTION: &str = "template.window.update";
const SECONDARY_COMPLETE_ACTION: &str = "template.window.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn generated_project_completes_the_fixed_authenticated_multi_window_walkthrough() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-multi-window-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-multi-window",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-multi-window-session-app",
            "Generated Multi-Window Session App",
        ])
        .output()
        .expect("run native multi-window application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid multi-window input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native multi-window project");
    assert!(
        built.success(),
        "generated native multi-window project must build"
    );
    let executable = target_directory
        .join("release")
        .join("generated-multi-window-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated multi-window executable"
    );

    let primary_documents = UiDocumentMailbox::new();
    let primary_input = UiInputMailbox::new();
    let group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
        primary_documents.clone(),
        primary_input.clone(),
    );
    let close_signal = SessionCloseSignal::default();
    let policy = HostPolicy::new(
        "anodrel.native-multi-window-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::WindowOpen,
            Capability::WindowClose,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed multi-window template policy is valid");
    let (server, invitation) =
        WindowsPipeServer::create_with_session_window_group_and_service_bundle(
            policy,
            "native-multi-window-template-session",
            group.clone(),
            close_signal.clone(),
            HostServices::unavailable(),
        )
        .expect("create fixed multi-window template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert multi-window invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated multi-window child");

    let primary = wait_for_document(&primary_documents, &child);
    assert_eq!(primary.revision().value(), 1);
    primary_input.push(UiInputCandidate::new(
        primary.revision(),
        UiEvent::ActionInvoked(
            ElementId::new(PRIMARY_OPEN_ACTION).expect("fixed primary action is valid"),
        ),
    ));

    let open = wait_for_open(&group, &child);
    assert_eq!(open.snapshot().snapshot().revision().value(), 1);
    let secondary_resources = open.resources().clone();
    let secondary = secondary_resources.id().clone();
    assert!(matches!(secondary, UiWindowId::Secondary(_)));
    assert!(
        group.complete_open(open.id(), true),
        "the simulated host commits the secondary after native registration"
    );
    let secondary_initial = wait_for_document(&secondary_resources.document_mailbox(), &child);
    assert_eq!(secondary_initial.revision().value(), 1);
    secondary_resources
        .input_mailbox()
        .push(UiInputCandidate::new(
            secondary_initial.revision(),
            UiEvent::ActionInvoked(
                ElementId::new(SECONDARY_UPDATE_ACTION)
                    .expect("fixed secondary update action is valid"),
            ),
        ));

    let secondary_updated = wait_for_document(&secondary_resources.document_mailbox(), &child);
    assert_eq!(secondary_updated.revision().value(), 2);
    secondary_resources
        .input_mailbox()
        .push(UiInputCandidate::new(
            secondary_updated.revision(),
            UiEvent::ActionInvoked(
                ElementId::new(SECONDARY_COMPLETE_ACTION)
                    .expect("fixed secondary complete action is valid"),
            ),
        ));

    let close_requests = wait_for_secondary_close(&group, &child);
    assert_eq!(close_requests, vec![secondary.clone()]);
    assert!(
        group.close_secondary(&secondary).is_ok(),
        "the simulated native destroy releases only the exact secondary"
    );
    assert!(
        close_signal.take(),
        "the generated project ends only its own authenticated session"
    );
    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated multi-window template exits within its bound"),
        0,
        "generated multi-window template must complete each fixed stage"
    );
    worker
        .join()
        .expect("multi-window template pipe worker does not panic")
        .expect("multi-window template pipe worker completes");
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
        assert_child_is_running(child, "delivering a document");
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated multi-window template did not deliver a document within its bound");
}

fn wait_for_open(
    group: &UiWindowGroup<WindowTitleProposal>,
    child: &LaunchedProcess,
) -> anodrel_ui_session::UiWindowOpenRequest<WindowTitleProposal> {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(request) = group.take_open_request() {
            return request;
        }
        assert_child_is_running(child, "requesting a secondary view");
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated multi-window template did not request a secondary within its bound");
}

fn wait_for_secondary_close(
    group: &UiWindowGroup<WindowTitleProposal>,
    child: &LaunchedProcess,
) -> Vec<UiWindowId> {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        let requests = group.take_secondary_close_requests();
        if !requests.is_empty() {
            return requests;
        }
        assert_child_is_running(child, "requesting secondary close");
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated multi-window template did not request secondary close within its bound");
}

fn assert_child_is_running(child: &LaunchedProcess, stage: &str) {
    if let Ok(exit_code) = child.wait_for_exit(0) {
        panic!("generated multi-window template stopped at safe stage {exit_code} before {stage}");
    }
}
