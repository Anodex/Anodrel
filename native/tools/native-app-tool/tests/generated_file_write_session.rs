//! End-to-end proof for the executable project created by the file-write generator.

#![forbid(unsafe_code)]

mod support;

use std::{
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use support::TestDirectory;

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_file_access::{
    FileTextWriteService, FileTextWriteServiceError, SaveReference, SaveSelection,
    SaveSelectionResult, SaveSelectionService, SaveSelectionServiceError,
};
use anodrel_file_dialog::{FileDialogFilter, SaveFilePath};
use anodrel_protocol::Capability;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const OUTPUT_TEXT: &str = "Hello from Anodrel's retained native file-write template.\n";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 10_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);
const REFERENCE: &str = "AbCdEfGhIjKlMnOpQrStUv";

#[derive(Clone, Debug)]
struct TestSaveSelectionService {
    selection: SaveSelection,
    filters: Arc<Mutex<Vec<Vec<FileDialogFilter>>>>,
}

impl TestSaveSelectionService {
    fn new(selection: SaveSelection) -> Self {
        Self {
            selection,
            filters: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn received_filters(&self) -> Vec<Vec<FileDialogFilter>> {
        self.filters
            .lock()
            .expect("test filter history remains available")
            .clone()
    }
}

impl SaveSelectionService for TestSaveSelectionService {
    fn save_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, SaveSelectionServiceError> {
        self.filters
            .lock()
            .expect("test filter history remains available")
            .push(filters.to_vec());
        Ok(SaveSelectionResult::Selected(self.selection.clone()))
    }
}

#[derive(Clone, Debug, Default)]
struct TestTextWriteService {
    writes: Arc<Mutex<Vec<(SaveReference, String)>>>,
}

impl TestTextWriteService {
    fn writes(&self) -> Vec<(SaveReference, String)> {
        self.writes
            .lock()
            .expect("test write history remains available")
            .clone()
    }
}

impl FileTextWriteService for TestTextWriteService {
    fn write_text(
        &self,
        reference: &SaveReference,
        text: &str,
    ) -> Result<(), FileTextWriteServiceError> {
        self.writes
            .lock()
            .expect("test write history remains available")
            .push((reference.clone(), text.to_owned()));
        Ok(())
    }
}

#[test]
fn generated_file_write_project_completes_one_retained_authenticated_session() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-file-write-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-file-write",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-file-write-session-app",
            "Generated File Write Session App",
        ])
        .output()
        .expect("run native file-write application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid file-write input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native file-write project");
    assert!(
        built.success(),
        "generated native file-write project must build"
    );
    let executable = target_directory
        .join("release")
        .join("generated-file-write-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated file-write executable"
    );

    let path = SaveFilePath::new("C:\\Anodrel\\generated-output.txt")
        .expect("fixed display path is valid");
    let reference = SaveReference::new(REFERENCE).expect("fixed save reference is valid");
    let selection_service = TestSaveSelectionService::new(SaveSelection::new(path, reference));
    let write_service = TestTextWriteService::default();
    let document_mailbox = UiDocumentMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let policy = HostPolicy::new(
        "anodrel.native-file-write-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::DialogSaveFile,
            Capability::FileWriteText,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed file-write template policy is valid");
    let services = HostServices::unavailable()
        .with_file_save_selections(selection_service.clone())
        .with_file_text_write(write_service.clone());
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            "native-file-write-template-session",
            document_mailbox.clone(),
            UiInputMailbox::new(),
            close_signal.clone(),
            services,
        )
        .expect("create fixed file-write template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert file-write template invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child = launch(&command, &bootstrap).expect("launch generated file-write template child");

    let snapshot = wait_for_document(&document_mailbox, &child);
    assert_eq!(snapshot.revision().value(), 1);
    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated file-write template exits within its bound"),
        0,
        "generated file-write template must complete rather than stop at a safe stage"
    );
    assert_eq!(
        selection_service.received_filters(),
        vec![vec![
            FileDialogFilter::new("Text documents", vec!["txt".to_owned()])
                .expect("fixed filter is valid"),
        ]]
    );
    assert_eq!(
        write_service.writes(),
        vec![(
            SaveReference::new(REFERENCE).expect("fixed save reference is valid"),
            OUTPUT_TEXT.to_owned(),
        )]
    );
    assert!(
        close_signal.take(),
        "generated file-write template must request close only for its own session"
    );
    worker
        .join()
        .expect("file-write-template pipe worker does not panic")
        .expect("file-write-template pipe worker completes");
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
                "generated file-write template stopped at safe stage {exit_code} before delivering its document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated file-write template did not deliver its document within its bound");
}
