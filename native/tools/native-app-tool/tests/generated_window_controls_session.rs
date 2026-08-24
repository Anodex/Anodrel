//! End-to-end proof for the generated native window-controls template.

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
use anodrel_ui::{ElementId, UiEvent};
use anodrel_ui_session::{UiDocumentMailbox, UiDocumentSnapshot, UiInputCandidate, UiInputMailbox};
use anodrel_window::{
    WindowFocusMailbox, WindowFullscreenMailbox, WindowFullscreenMode, WindowSizeMailbox,
    WindowState, WindowStateMailbox, WindowTitleMailbox,
};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const TITLE_ACTION: &str = "template.controls.title";
const RESIZE_ACTION: &str = "template.controls.resize";
const MAXIMIZE_ACTION: &str = "template.controls.maximize";
const RESTORE_ACTION: &str = "template.controls.restore";
const FOCUS_ACTION: &str = "template.controls.focus";
const FULLSCREEN_ACTION: &str = "template.controls.fullscreen";
const WINDOWED_ACTION: &str = "template.controls.windowed";
const COMPLETE_ACTION: &str = "template.controls.complete";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const REQUEST_WAIT: Duration = Duration::from_secs(20);
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
            "anodrel-native-window-controls-session-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create unique native window-controls test directory");
        Self { path }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let temporary = std::env::temp_dir();
        let expected_prefix = format!(
            "anodrel-native-window-controls-session-test-{}-",
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
fn generated_project_completes_every_fixed_window_control_in_one_authenticated_session() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-window-controls-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-window-controls",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-window-controls-session-app",
            "Generated Window Controls Session App",
        ])
        .output()
        .expect("run native window-controls application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid window-controls input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native window-controls project");
    assert!(
        built.success(),
        "generated native window-controls project must build"
    );
    let executable = target_directory
        .join("release")
        .join("generated-window-controls-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated window-controls executable"
    );

    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let title_mailbox = WindowTitleMailbox::new();
    let state_mailbox = WindowStateMailbox::new();
    let focus_mailbox = WindowFocusMailbox::new();
    let fullscreen_mailbox = WindowFullscreenMailbox::new();
    let size_mailbox = WindowSizeMailbox::new();
    let policy = HostPolicy::new(
        "anodrel.native-window-controls-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::WindowTitle,
            Capability::WindowState,
            Capability::WindowFocus,
            Capability::WindowFullscreen,
            Capability::WindowSize,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed window-controls policy is valid");
    let services = HostServices::unavailable()
        .with_window_title(title_mailbox.clone())
        .with_window_state(state_mailbox.clone())
        .with_window_focus(focus_mailbox.clone())
        .with_window_fullscreen(fullscreen_mailbox.clone())
        .with_window_size(size_mailbox.clone());
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            "native-window-controls-template-session",
            document_mailbox.clone(),
            input_mailbox.clone(),
            close_signal.clone(),
            services,
        )
        .expect("create fixed window-controls test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert window-controls invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated window-controls child");

    send_action(
        &input_mailbox,
        &wait_for_document(&document_mailbox, &child, 1),
        TITLE_ACTION,
    );
    let request = wait_for_request(&child, "title", || title_mailbox.take());
    assert_eq!(request.proposal().as_str(), "Window controls exercised");
    assert!(title_mailbox.complete(request.id()));

    send_action(
        &input_mailbox,
        &wait_for_document(&document_mailbox, &child, 2),
        RESIZE_ACTION,
    );
    let request = wait_for_request(&child, "size", || size_mailbox.take());
    assert_eq!(request.size().width(), 960);
    assert_eq!(request.size().height(), 640);
    assert!(size_mailbox.complete(request.id()));

    send_action(
        &input_mailbox,
        &wait_for_document(&document_mailbox, &child, 3),
        MAXIMIZE_ACTION,
    );
    let request = wait_for_request(&child, "maximize state", || state_mailbox.take());
    assert_eq!(request.state(), WindowState::Maximized);
    assert!(state_mailbox.complete(request.id()));

    send_action(
        &input_mailbox,
        &wait_for_document(&document_mailbox, &child, 4),
        RESTORE_ACTION,
    );
    let request = wait_for_request(&child, "restore state", || state_mailbox.take());
    assert_eq!(request.state(), WindowState::Restored);
    assert!(state_mailbox.complete(request.id()));

    send_action(
        &input_mailbox,
        &wait_for_document(&document_mailbox, &child, 5),
        FOCUS_ACTION,
    );
    let request = wait_for_request(&child, "focus", || focus_mailbox.take());
    assert!(focus_mailbox.complete(request.id()));

    send_action(
        &input_mailbox,
        &wait_for_document(&document_mailbox, &child, 6),
        FULLSCREEN_ACTION,
    );
    let request = wait_for_request(&child, "fullscreen", || fullscreen_mailbox.take());
    assert_eq!(request.mode(), WindowFullscreenMode::Fullscreen);
    assert!(fullscreen_mailbox.complete(request.id()));

    send_action(
        &input_mailbox,
        &wait_for_document(&document_mailbox, &child, 7),
        WINDOWED_ACTION,
    );
    let request = wait_for_request(&child, "windowed", || fullscreen_mailbox.take());
    assert_eq!(request.mode(), WindowFullscreenMode::Windowed);
    assert!(fullscreen_mailbox.complete(request.id()));

    send_action(
        &input_mailbox,
        &wait_for_document(&document_mailbox, &child, 8),
        COMPLETE_ACTION,
    );
    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated window-controls template exits within its bound"),
        0,
        "generated window-controls template must complete every fixed stage"
    );
    assert!(
        close_signal.take(),
        "generated window-controls template must request close only for its own session"
    );
    worker
        .join()
        .expect("window-controls pipe worker does not panic")
        .expect("window-controls pipe worker completes");
}

fn send_action(mailbox: &UiInputMailbox, snapshot: &UiDocumentSnapshot, action: &str) {
    mailbox.push(UiInputCandidate::new(
        snapshot.revision(),
        UiEvent::ActionInvoked(ElementId::new(action).expect("fixed action is valid")),
    ));
}

fn wait_for_document(
    mailbox: &UiDocumentMailbox,
    child: &LaunchedProcess,
    expected_revision: u64,
) -> UiDocumentSnapshot {
    let snapshot = wait_for_request(child, "document", || mailbox.take());
    assert_eq!(
        snapshot.revision().value(),
        expected_revision,
        "generated window-controls document advances through its fixed order"
    );
    snapshot
}

fn wait_for_request<T>(
    child: &LaunchedProcess,
    stage: &str,
    mut take: impl FnMut() -> Option<T>,
) -> T {
    let deadline = Instant::now() + REQUEST_WAIT;
    while Instant::now() < deadline {
        if let Some(request) = take() {
            return request;
        }
        if let Ok(exit_code) = child.wait_for_exit(0) {
            panic!(
                "generated window-controls template stopped at safe stage {exit_code} before {stage}"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated window-controls template did not deliver {stage} within its bound");
}
