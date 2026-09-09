//! Explicit unavailable implementations for core-service defaults.
//!
//! Each type accepts its documented input shape but refuses every operation.
//! Keeping these sentinels separate from bundle construction makes the default
//! authority boundary straightforward to audit.

use super::*;

#[derive(Debug)]
pub(crate) struct UnavailableClipboard;

impl ClipboardService for UnavailableClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }

    fn write_text(&self, _text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableExternalLinks;

impl ExternalLinkService for UnavailableExternalLinks {
    fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        Err(ExternalLinkOpenError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableNetwork;

impl NetworkTextService for UnavailableNetwork {
    fn fetch_text(
        &self,
        _url: &NetworkUrl,
    ) -> Result<anodrel_network::NetworkTextResponse, NetworkTextServiceError> {
        Err(NetworkTextServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableNotifications;

impl NotificationService for UnavailableNotifications {
    fn show(&self, _notification: &Notification) -> Result<(), NotificationServiceError> {
        Err(NotificationServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableUiFields;

impl UiFieldReader for UnavailableUiFields {
    fn read(&self) -> Result<UiFieldSnapshot, UiFieldReadError> {
        Err(UiFieldReadError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableWindowTitle;

impl WindowTitleService for UnavailableWindowTitle {
    fn set_title(&self, _proposal: &WindowTitleProposal) -> Result<(), WindowTitleServiceError> {
        Err(WindowTitleServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableWindowState;

impl WindowStateService for UnavailableWindowState {
    fn set_state(&self, _state: WindowState) -> Result<(), WindowStateServiceError> {
        Err(WindowStateServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableWindowStateRead;

impl WindowStateReadService for UnavailableWindowStateRead {
    fn read_state(&self) -> Result<WindowState, WindowStateReadServiceError> {
        Err(WindowStateReadServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableWindowStateChanges;

impl WindowStateChangesService for UnavailableWindowStateChanges {
    fn read_change(&self) -> Result<Option<WindowState>, WindowStateChangesServiceError> {
        Err(WindowStateChangesServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableWindowFocus;

impl WindowFocusService for UnavailableWindowFocus {
    fn request_focus(&self) -> Result<(), WindowFocusServiceError> {
        Err(WindowFocusServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableWindowFullscreen;

impl WindowFullscreenService for UnavailableWindowFullscreen {
    fn set_fullscreen(
        &self,
        _mode: WindowFullscreenMode,
    ) -> Result<(), WindowFullscreenServiceError> {
        Err(WindowFullscreenServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableWindowSize;

impl WindowSizeService for UnavailableWindowSize {
    fn set_size(&self, _size: WindowSize) -> Result<(), WindowSizeServiceError> {
        Err(WindowSizeServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableFileDialogs;

impl FileDialogService for UnavailableFileDialogs {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableStorage;

impl StorageService for UnavailableStorage {
    fn read(&self) -> Result<StorageRead, StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }

    fn replace(&self, _snapshot: &StorageSnapshot) -> Result<(), StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }

    fn clear(&self) -> Result<(), StorageServiceError> {
        Err(StorageServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableDiagnostics;

impl DiagnosticsService for UnavailableDiagnostics {
    fn entries(&self) -> Result<Vec<anodrel_diagnostics::Entry>, DiagnosticsServiceError> {
        Err(DiagnosticsServiceError::Unavailable)
    }
}

#[derive(Debug)]
pub(crate) struct UnavailableCredentials;

impl CredentialService for UnavailableCredentials {
    fn read(&self, _name: &CredentialName) -> Result<Secret, CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }

    fn write(
        &self,
        _name: &CredentialName,
        _secret: &Secret,
    ) -> Result<(), CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }

    fn delete(&self, _name: &CredentialName) -> Result<bool, CredentialServiceError> {
        Err(CredentialServiceError::Unavailable)
    }
}
