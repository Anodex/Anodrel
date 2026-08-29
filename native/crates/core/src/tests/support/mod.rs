//! Shared test fixtures and request builders.

mod documents;
mod hosts;
mod notifications;
mod requests;

pub(super) use documents::{
    field, ui_document_payload, valid_ui_document, valid_ui_document_v2, valid_ui_document_v3,
};
pub(super) use hosts::{
    clipboard_host, credential_host, external_host, file_access_host, file_binary_write_host,
    file_dialog_host, file_write_host, network_host, storage_host,
};
pub(super) use notifications::{
    RecordingNotifications, host_with_notifications, notification_payload,
};
pub(super) use requests::{
    request, request_v1_1, request_v1_2, request_v1_3, request_v1_4, request_v1_5, request_v1_6,
    request_v1_7, request_v1_8, request_v1_9, request_v1_10, request_v1_12, request_v1_13,
    request_v1_15, request_v1_28, request_v1_29, request_v1_30, request_v1_31,
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
pub(super) use anodrel_file_dialog::{
    FileDialogFilter, FileDialogSelection, FileDialogService, FileDialogServiceError, SaveFilePath,
    SelectedFilePath, SelectedFolderPath,
};
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
    MenuInputCandidate, UiDocumentMailbox, UiInputCandidate, UiInputMailbox,
};
pub(super) use anodrel_window::{
    WindowFocusService, WindowFocusServiceError, WindowFullscreenMode, WindowFullscreenService,
    WindowFullscreenServiceError, WindowSize, WindowSizeService, WindowSizeServiceError,
    WindowState, WindowStateChangesService, WindowStateChangesServiceError, WindowStateReadService,
    WindowStateReadServiceError, WindowStateService, WindowStateServiceError, WindowTitleProposal,
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

pub(crate) mod windows;
pub(crate) use windows::*;
