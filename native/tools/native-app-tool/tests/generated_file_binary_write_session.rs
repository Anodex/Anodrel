//! End-to-end proof for the executable project created by the binary-write generator.

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
    FileBinaryData, FileBinaryWriteService, FileBinaryWriteServiceError, SaveReference,
    SaveSelection, SaveSelectionResult, SaveSelectionService, SaveSelectionServiceError,
};
use anodrel_file_dialog::{FileDialogFilter, SaveFilePath};
use anodrel_protocol::Capability;
use anodrel_ui_session::{UiDocumentMailbox, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, LaunchedProcess, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const BINARY_BYTES: &[u8] = &[0x41, 0x6E, 0x6F, 0x64, 0x72, 0x65, 0x6C, 0x00, 0xFF];
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
struct TestBinaryWriteService {
    writes: Arc<Mutex<Vec<(SaveReference, Vec<u8>)>>>,
    discarded: Arc<Mutex<Vec<SaveReference>>>,
}

impl TestBinaryWriteService {
    fn writes(&self) -> Vec<(SaveReference, Vec<u8>)> {
        self.writes
            .lock()
            .expect("test write history remains available")
            .clone()
    }

    fn discarded(&self) -> Vec<SaveReference> {
        self.discarded
            .lock()
            .expect("test discard history remains available")
            .clone()
    }
}

impl FileBinaryWriteService for TestBinaryWriteService {
    fn write_binary(
        &self,
        reference: &SaveReference,
        data: &FileBinaryData,
    ) -> Result<(), FileBinaryWriteServiceError> {
        self.writes
            .lock()
            .expect("test write history remains available")
            .push((reference.clone(), data.as_bytes().to_vec()));
        Ok(())
    }

    fn discard(&self, reference: &SaveReference) {
        self.discarded
            .lock()
            .expect("test discard history remains available")
            .push(reference.clone());
    }
}

#[test]
fn generated_binary_file_write_project_completes_one_retained_authenticated_session() {
    let temporary = TestDirectory::new();
    let destination = temporary
        .path
        .join("generated-file-binary-write-session-app");
    let generated = Command::new(env!("CARGO_BIN_EXE_anodrel-native-app-tool"))
        .args([
            "init-file-binary-write",
            destination.to_str().expect("test destination is UTF-8"),
            "generated-file-binary-write-session-app",
            "Generated File Binary Write Session App",
        ])
        .output()
        .expect("run native binary file-write application generator");
    assert!(
        generated.status.success(),
        "the generator must accept valid binary file-write input"
    );

    let target_directory = temporary.path.join("target");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--release", "--manifest-path"])
        .arg(destination.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_directory)
        .status()
        .expect("build generated native binary file-write project");
    assert!(
        built.success(),
        "generated native binary file-write project must build"
    );
    let executable = target_directory
        .join("release")
        .join("generated-file-binary-write-session-app.exe");
    assert!(
        executable.is_file(),
        "release build must produce the generated binary file-write executable"
    );

    let path = SaveFilePath::new("C:\\Anodrel\\generated-output.bin")
        .expect("fixed display path is valid");
    let reference = SaveReference::new(REFERENCE).expect("fixed save reference is valid");
    let selection_service = TestSaveSelectionService::new(SaveSelection::new(path, reference));
    let write_service = TestBinaryWriteService::default();
    let document_mailbox = UiDocumentMailbox::new();
    let close_signal = SessionCloseSignal::default();
    let policy = HostPolicy::new(
        "anodrel.native-file-binary-write-template",
        vec![
            Capability::UiDocumentWrite,
            Capability::DialogSaveFile,
            Capability::FileWriteBinary,
            Capability::SessionClose,
        ],
        "anodrel-native-app-tool-test-host",
    )
    .expect("fixed binary file-write template policy is valid");
    let services = HostServices::unavailable()
        .with_file_save_selections(selection_service.clone())
        .with_file_binary_write(write_service.clone());
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            "native-file-binary-write-template-session",
            document_mailbox.clone(),
            UiInputMailbox::new(),
            close_signal.clone(),
            services,
        )
        .expect("create fixed binary file-write template test pipe");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("convert binary file-write template invitation to bootstrap record");
    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(executable.to_str().expect("test executable is UTF-8"))
        .expect("generated executable path is valid");
    let child =
        launch(&command, &bootstrap).expect("launch generated binary file-write template child");

    let snapshot = wait_for_document(&document_mailbox, &child);
    assert_eq!(snapshot.revision().value(), 1);
    assert_eq!(
        child
            .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
            .expect("generated binary file-write template exits within its bound"),
        0,
        "generated binary file-write template must complete rather than stop at a safe stage"
    );
    assert_eq!(
        selection_service.received_filters(),
        vec![vec![
            FileDialogFilter::new("Binary files", vec!["bin".to_owned()])
                .expect("fixed filter is valid"),
        ]]
    );
    assert_eq!(
        write_service.writes(),
        vec![(
            SaveReference::new(REFERENCE).expect("fixed save reference is valid"),
            BINARY_BYTES.to_vec(),
        )]
    );
    assert!(write_service.discarded().is_empty());
    assert!(
        close_signal.take(),
        "generated binary file-write template must request close only for its own session"
    );
    worker
        .join()
        .expect("binary-file-write-template pipe worker does not panic")
        .expect("binary-file-write-template pipe worker completes");
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
                "generated binary file-write template stopped at safe stage {exit_code} before delivering its document"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("generated binary file-write template did not deliver its document within its bound");
}
