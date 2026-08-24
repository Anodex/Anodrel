//! Shared test fixtures and request builders.

mod requests;

pub(super) use requests::{
    request, request_v1_1, request_v1_2, request_v1_3, request_v1_4, request_v1_5, request_v1_6,
    request_v1_7, request_v1_8, request_v1_9, request_v1_10, request_v1_12, request_v1_13,
    request_v1_15,
};

pub(super) use std::cell::RefCell;
pub(super) use std::collections::BTreeMap;
pub(super) use std::sync::{Arc, Mutex};
pub(super) use std::thread;

pub(super) use anodrel_clipboard::{
    ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText,
};
pub(super) use anodrel_credentials::{
    CredentialName, CredentialService, CredentialServiceError, Secret,
};
pub(super) use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
pub(super) use anodrel_file_access::{
    FileBinaryData, FileBinaryWriteService, FileBinaryWriteServiceError, FileSelection,
    FileSelectionService, FileTextService, FileTextWriteService, FileTextWriteServiceError,
    SaveReference, SaveSelection, SaveSelectionResult, SaveSelectionService,
    SaveSelectionServiceError,
};
pub(super) use anodrel_file_dialog::{SaveFilePath, SelectedFilePath};
pub(super) use anodrel_menu::{MenuModel, MenuRevision, MenuService, MenuServiceError};
pub(super) use anodrel_network::{
    NetworkTextResponse, NetworkTextService, NetworkTextServiceError, NetworkUrl,
};
pub(super) use anodrel_notifications::{
    Notification, NotificationService, NotificationServiceError,
};
pub(super) use anodrel_storage::{
    StorageRead, StorageService, StorageServiceError, StorageSnapshot,
};
pub(super) use anodrel_ui::{ElementId, UiEvent};
pub(super) use anodrel_ui_session::{
    MenuInputCandidate, UiDocumentMailbox, UiInputCandidate, UiInputMailbox, UiWindowGroup,
};
pub(super) use anodrel_window::{
    WindowFocusService, WindowFocusServiceError, WindowFullscreenMode, WindowFullscreenService,
    WindowFullscreenServiceError, WindowSize, WindowSizeService, WindowSizeServiceError,
    WindowState, WindowStateService, WindowStateServiceError, WindowTitleProposal,
    WindowTitleService, WindowTitleServiceError,
};

use crate::*;

pub(super) fn host(grants: Vec<Capability>) -> CoreHost {
    CoreHost::new(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
    )
}

#[derive(Debug)]
pub(super) struct MemoryClipboard {
    pub(super) text: RefCell<Option<ClipboardText>>,
}

impl MemoryClipboard {
    pub(super) fn with_text(text: Option<&str>) -> Self {
        Self {
            text: RefCell::new(text.map(|value| ClipboardText::new(value).expect("fixture text"))),
        }
    }
}

impl ClipboardService for MemoryClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        Ok(self
            .text
            .borrow()
            .clone()
            .map(ClipboardRead::Text)
            .unwrap_or(ClipboardRead::NoText))
    }

    fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        *self.text.borrow_mut() = Some(text.clone());
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct FailingClipboard(pub(super) ClipboardServiceError);

impl ClipboardService for FailingClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        Err(self.0)
    }

    fn write_text(&self, _text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        Err(self.0)
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingExternalLinks(pub(super) RefCell<Option<ExternalLink>>);

impl ExternalLinkService for RecordingExternalLinks {
    fn open(&self, link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        *self.0.borrow_mut() = Some(link.clone());
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct FailingExternalLinks;

impl ExternalLinkService for FailingExternalLinks {
    fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        Err(ExternalLinkOpenError::Unavailable)
    }
}

#[derive(Clone, Debug)]
pub(super) struct RecordingNetwork {
    pub(super) requested: Arc<Mutex<Vec<NetworkUrl>>>,
    pub(super) result: Result<NetworkTextResponse, NetworkTextServiceError>,
}

impl RecordingNetwork {
    pub(super) fn responding(status_code: u16, text: &str) -> Self {
        Self {
            requested: Arc::new(Mutex::new(Vec::new())),
            result: Ok(NetworkTextResponse::new(status_code, text)
                .expect("network fixture response is valid")),
        }
    }

    pub(super) fn failing(error: NetworkTextServiceError) -> Self {
        Self {
            requested: Arc::new(Mutex::new(Vec::new())),
            result: Err(error),
        }
    }
}

impl NetworkTextService for RecordingNetwork {
    fn fetch_text(&self, url: &NetworkUrl) -> Result<NetworkTextResponse, NetworkTextServiceError> {
        self.requested
            .lock()
            .expect("network recorder lock is available")
            .push(url.clone());
        self.result.clone()
    }
}

#[derive(Debug)]
pub(super) struct CancellingFileDialog;

impl FileDialogService for CancellingFileDialog {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Ok(FileDialogSelection::Cancelled)
    }
}

#[derive(Debug)]
pub(super) struct SavingFileDialog;

impl FileDialogService for SavingFileDialog {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Ok(FileDialogSelection::Cancelled)
    }

    fn save_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        let destination = std::env::temp_dir().join("anodrel-save-dialog-test.txt");
        Ok(FileDialogSelection::Saved(
            SaveFilePath::new(destination).expect("temporary directory is absolute"),
        ))
    }
}

#[derive(Debug)]
pub(super) struct CapturingFileDialog;

impl FileSelectionService for CapturingFileDialog {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileSelectionResult, FileSelectionServiceError> {
        let path =
            SelectedFilePath::new(r"C:\\Users\\Owner\\selection.txt").expect("path is valid");
        let reference =
            SelectionReference::new("AbCdEfGhIjKlMnOpQrStUv").expect("reference is valid");
        Ok(FileSelectionResult::Selected(FileSelection::new(
            path, reference,
        )))
    }
}

#[derive(Debug)]
pub(super) struct FixedFileText(pub(super) Result<String, FileTextServiceError>);

impl FileTextService for FixedFileText {
    fn read_text(&self, _reference: &SelectionReference) -> Result<String, FileTextServiceError> {
        self.0.clone()
    }
}

#[derive(Debug)]
pub(super) struct CapturingSaveDialog;

impl SaveSelectionService for CapturingSaveDialog {
    fn save_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, SaveSelectionServiceError> {
        let path = SaveFilePath::new(r"C:\\Users\\Owner\\save.txt").expect("path is valid");
        let reference = SaveReference::new("ZyXwVuTsRqPoNmLkJiHgFe").expect("reference is valid");
        Ok(SaveSelectionResult::Selected(SaveSelection::new(
            path, reference,
        )))
    }
}

#[derive(Clone, Debug)]
pub(super) struct RecordingFileTextWrite {
    pub(super) writes: Arc<Mutex<Vec<String>>>,
    pub(super) result: Result<(), FileTextWriteServiceError>,
}

impl RecordingFileTextWrite {
    pub(super) fn accepting() -> Self {
        Self {
            writes: Arc::new(Mutex::new(Vec::new())),
            result: Ok(()),
        }
    }

    pub(super) fn failing(error: FileTextWriteServiceError) -> Self {
        Self {
            writes: Arc::new(Mutex::new(Vec::new())),
            result: Err(error),
        }
    }
}

impl FileTextWriteService for RecordingFileTextWrite {
    fn write_text(
        &self,
        _reference: &SaveReference,
        text: &str,
    ) -> Result<(), FileTextWriteServiceError> {
        self.writes
            .lock()
            .expect("write recorder lock is available")
            .push(text.to_owned());
        self.result
    }
}

#[derive(Clone, Debug)]
pub(super) struct RecordingFileBinaryWrite {
    pub(super) writes: Arc<Mutex<Vec<Vec<u8>>>>,
    pub(super) discarded: Arc<Mutex<Vec<SaveReference>>>,
    pub(super) result: Result<(), FileBinaryWriteServiceError>,
}

impl RecordingFileBinaryWrite {
    pub(super) fn accepting() -> Self {
        Self {
            writes: Arc::new(Mutex::new(Vec::new())),
            discarded: Arc::new(Mutex::new(Vec::new())),
            result: Ok(()),
        }
    }

    pub(super) fn unavailable() -> Self {
        Self {
            writes: Arc::new(Mutex::new(Vec::new())),
            discarded: Arc::new(Mutex::new(Vec::new())),
            result: Err(FileBinaryWriteServiceError::Unavailable),
        }
    }
}

impl FileBinaryWriteService for RecordingFileBinaryWrite {
    fn write_binary(
        &self,
        _reference: &SaveReference,
        data: &FileBinaryData,
    ) -> Result<(), FileBinaryWriteServiceError> {
        self.writes
            .lock()
            .expect("binary-write recorder lock is available")
            .push(data.as_bytes().to_vec());
        self.result
    }

    fn discard(&self, reference: &SaveReference) {
        self.discarded
            .lock()
            .expect("binary-discard recorder lock is available")
            .push(reference.clone());
    }
}

#[derive(Clone, Debug)]
pub(super) struct RecordingMenu {
    pub(super) replacements: Arc<Mutex<Vec<(MenuRevision, MenuModel)>>>,
    pub(super) result: Result<(), MenuServiceError>,
}

impl Default for RecordingMenu {
    fn default() -> Self {
        Self {
            replacements: Arc::new(Mutex::new(Vec::new())),
            result: Ok(()),
        }
    }
}

impl RecordingMenu {
    pub(super) fn unavailable() -> Self {
        Self {
            replacements: Arc::new(Mutex::new(Vec::new())),
            result: Err(MenuServiceError::Unavailable),
        }
    }
}

impl MenuService for RecordingMenu {
    fn replace(&self, revision: MenuRevision, model: MenuModel) -> Result<(), MenuServiceError> {
        if self.result.is_ok() {
            self.replacements
                .lock()
                .expect("menu recorder lock is available")
                .push((revision, model));
        }
        self.result
    }
}

#[derive(Debug)]
pub(super) struct MemoryStorage(Mutex<Result<StorageRead, StorageServiceError>>);

impl MemoryStorage {
    pub(super) fn with_state(state: StorageRead) -> Self {
        Self(Mutex::new(Ok(state)))
    }
}

impl StorageService for MemoryStorage {
    fn read(&self) -> Result<StorageRead, StorageServiceError> {
        self.0.lock().expect("storage lock is available").clone()
    }

    fn replace(&self, snapshot: &StorageSnapshot) -> Result<(), StorageServiceError> {
        *self.0.lock().expect("storage lock is available") =
            Ok(StorageRead::Snapshot(snapshot.clone()));
        Ok(())
    }

    fn clear(&self) -> Result<(), StorageServiceError> {
        *self.0.lock().expect("storage lock is available") = Ok(StorageRead::Absent);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(super) struct MemoryCredentials(Mutex<BTreeMap<String, Vec<u8>>>);

impl CredentialService for MemoryCredentials {
    fn read(&self, name: &CredentialName) -> Result<Secret, CredentialServiceError> {
        self.0
            .lock()
            .expect("credential lock is available")
            .get(name.as_str())
            .cloned()
            .map(|bytes| Secret::new(bytes).expect("stored fixture secret is valid"))
            .ok_or(CredentialServiceError::NotFound)
    }

    fn write(&self, name: &CredentialName, secret: &Secret) -> Result<(), CredentialServiceError> {
        self.0
            .lock()
            .expect("credential lock is available")
            .insert(name.as_str().to_owned(), secret.as_bytes().to_vec());
        Ok(())
    }

    fn delete(&self, name: &CredentialName) -> Result<bool, CredentialServiceError> {
        Ok(self
            .0
            .lock()
            .expect("credential lock is available")
            .remove(name.as_str())
            .is_some())
    }
}

pub(super) fn clipboard_host(
    grants: Vec<Capability>,
    clipboard: impl ClipboardService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_clipboard(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        clipboard,
    )
}

pub(super) fn external_host(
    grants: Vec<Capability>,
    external_links: impl ExternalLinkService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text(None),
        external_links,
    )
}

pub(super) fn network_host(
    grants: Vec<Capability>,
    network: impl NetworkTextService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        HostServices::unavailable().with_network(network),
    )
}

pub(super) fn file_dialog_host(
    grants: Vec<Capability>,
    dialogs: impl FileDialogService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_all_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text(None),
        FailingExternalLinks,
        dialogs,
    )
}

pub(super) fn file_access_host(
    grants: Vec<Capability>,
    selections: impl FileSelectionService + 'static,
    text: impl FileTextService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_all_services_and_file_access(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text(None),
        FailingExternalLinks,
        CancellingFileDialog,
        selections,
        text,
    )
}

pub(super) fn file_write_host(
    grants: Vec<Capability>,
    selections: impl SaveSelectionService + 'static,
    writer: impl FileTextWriteService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        HostServices::unavailable()
            .with_file_save_selections(selections)
            .with_file_text_write(writer),
    )
}

pub(super) fn file_binary_write_host(
    grants: Vec<Capability>,
    writer: impl FileBinaryWriteService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        HostServices::unavailable().with_file_binary_write(writer),
    )
}

pub(super) fn storage_host(
    grants: Vec<Capability>,
    storage: impl StorageService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_all_services_and_file_access_and_storage(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text(None),
        FailingExternalLinks,
        CancellingFileDialog,
        CapturingFileDialog,
        FixedFileText(Err(FileTextServiceError::Unavailable)),
        storage,
    )
}

pub(super) fn credential_host(
    grants: Vec<Capability>,
    credentials: impl CredentialService + 'static,
) -> CoreHost {
    CoreHost::with_credential_service(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        credentials,
    )
}

pub(super) fn host_with_notifications(service: impl NotificationService + 'static) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::NotificationShow],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_notifications(service),
    )
}

/// A notification service that records what it was asked to show.
#[derive(Debug, Default)]
pub(super) struct RecordingNotifications {
    pub(super) shown: std::sync::Mutex<Vec<(String, String)>>,
    pub(super) result: Option<NotificationServiceError>,
}

impl RecordingNotifications {
    pub(super) fn failing(error: NotificationServiceError) -> Self {
        Self {
            shown: std::sync::Mutex::new(Vec::new()),
            result: Some(error),
        }
    }
}

impl NotificationService for RecordingNotifications {
    fn show(&self, notification: &Notification) -> Result<(), NotificationServiceError> {
        if let Some(error) = self.result {
            return Err(error);
        }
        self.shown
            .lock()
            .expect("the fixture lock is usable")
            .push((
                notification.title().as_str().to_owned(),
                notification.body().as_str().to_owned(),
            ));
        Ok(())
    }
}

pub(super) fn notification_payload(title: &str, body: &str) -> String {
    object([
        ("body", JsonValue::String(body.to_owned())),
        ("title", JsonValue::String(title.to_owned())),
    ])
    .to_json()
}

pub(super) fn ui_document_payload(document: &str) -> String {
    object([("document", JsonValue::String(document.to_owned()))]).to_json()
}

pub(super) fn valid_ui_document(label: &str) -> String {
    format!(
        r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"root","kind":"action","label":"{label}","fontSize":16,"enabled":true,"tone":"accent"}}}}"#
    )
}

pub(super) fn valid_ui_document_v2() -> &'static str {
    r#"{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"content","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}}"#
}

pub(super) fn valid_ui_document_v3(value: &str, politeness: &str) -> String {
    format!(
        r#"{{"format":"anodrel.ui.document.v3","root":{{"id":"status","kind":"status","value":"{value}","fontSize":16,"tone":"accent","politeness":"{politeness}"}}}}"#
    )
}

pub(super) fn field<'a>(value: &'a JsonValue, field: &str) -> &'a JsonValue {
    &value.as_object().expect("response is an object")[field]
}

pub(crate) mod windows;
pub(crate) use windows::*;
