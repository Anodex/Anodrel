//! End-to-end proof for a generated native form project.

#![forbid(unsafe_code)]

use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_protocol::Capability;
use anodrel_ui::{
    Axis, ElementId, Field, Insets, Stack, UiDocument, UiEvent, UiFieldStates, UiNode,
};
use anodrel_ui_session::{
    UiDocumentMailbox, UiFieldMailbox, UiFieldRequest, UiFieldSnapshot, UiInputCandidate,
    UiInputMailbox,
};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const FORM_ACTION: &str = "template.form.submit";
const FORM_FIELD: &str = "template.form.name";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);
static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory {
    path: std::path::PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let temporary = std::env::temp_dir();
        let path = temporary.join(format!(
            "anodrel-native-form-app-session-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create unique native form test directory");
        Self { path }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let temporary = std::env::temp_dir();
        let expected_prefix = format!(
            "anodrel-native-form-app-session-test-{}-",
            std::process::id()
        );
        let name = self.path.file_name().and_then(|name| name.to_str());
        if self.path.parent() == Some(temporary.as_path())
            && name.is_some_and(|name| name.starts_with(&expected_prefix))
        {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[test]
fn generated_project_reads_one_whole_form_snapshot_after_submit() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-form-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-form",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-form-session-app",
            "Generated Form Session App",
        ])
        .output()
        .expect("run native form application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid form input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native form project");
    assert!(built.success(), "generated native form project must build");
    let executable = target_directory
        .join("release")
        .join("generated-form-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated form executable"
    );

    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let field_mailbox = UiFieldMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let policy = HostPolicy::new(
        "anodrel.native-form-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::UiFieldsRead,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed form template policy is valid");
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            "native-form-template-session",
            document_mailbox.clone(),
            input_mailbox.clone(),
            close_signal.clone(),
            HostServices::unavailable().with_ui_fields(field_mailbox.clone()),
        )
        .expect("create fixed form template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert form invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated form child");

    let document = wait_for_document(&document_mailbox, &child);
    assert_eq!(document.revision().value(), 1);
    input_mailbox.push(UiInputCandidate::new(
        document.revision(),
        UiEvent::ActionInvoked(ElementId::new(FORM_ACTION).expect("fixed form action is valid")),
    ));
    let request = wait_for_field_read(&field_mailbox, &child);
    assert!(
        field_mailbox.complete(request.id(), form_snapshot("Ada")),
        "the host returns one whole current form snapshot"
    );

    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated form template exits within its bound"),
        0,
        "generated form template completes each fixed stage"
    );
    assert!(
        close_signal.take(),
        "generated form template must close only its own session"
    );
    worker
        .join()
        .expect("form template pipe worker does not panic")
        .expect("form template pipe worker completes");
}

fn form_snapshot(value: &str) -> UiFieldSnapshot {
    let document = UiDocument::new(UiNode::Stack(
        Stack::new(
            ElementId::new("root").expect("test ID is valid"),
            Axis::Vertical,
            Insets::zero(),
            0,
            vec![UiNode::Field(
                Field::new(
                    ElementId::new(FORM_FIELD).expect("test field ID is valid"),
                    "Name",
                    value,
                    96,
                    16,
                    true,
                )
                .expect("test field is valid"),
            )],
        )
        .expect("test form stack is valid"),
    ))
    .expect("test form document is valid");
    let mut states = UiFieldStates::new();
    states.reseed(&document);
    UiFieldSnapshot::from_states(&states).expect("test form snapshot is bounded")
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
        assert_child_is_running(child, "delivering its form document");
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated form template did not deliver a document within its bound");
}

fn wait_for_field_read(mailbox: &UiFieldMailbox, child: &LaunchedProcess) -> UiFieldRequest {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(request) = mailbox.take() {
            return request;
        }
        assert_child_is_running(child, "requesting field values");
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated form template did not request fields within its bound");
}

fn assert_child_is_running(child: &LaunchedProcess, stage: &str) {
    if let Ok(exit_code) = child.wait_for_exit(0) {
        panic!("generated form template stopped at safe stage {exit_code} before {stage}");
    }
}
