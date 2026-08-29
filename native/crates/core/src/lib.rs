#![forbid(unsafe_code)]

//! Policy-bound handling for one native protocol message.
//!
//! Transports authenticate their sessions before calling this module. Incoming
//! capability context is intentionally ignored: only the host-created policy
//! can authorize a privileged operation.

mod construction;
mod dispatch;
mod file_access;
mod integrations;
mod persistence;
mod services;
mod ui_documents;
mod ui_interactions;
mod window;

pub use services::HostServices;
use services::{
    UnavailableClipboard, UnavailableCredentials, UnavailableDiagnostics, UnavailableExternalLinks,
    UnavailableFileDialogs, UnavailableNetwork, UnavailableNotifications, UnavailableStorage,
    UnavailableUiFields, UnavailableWindowFocus, UnavailableWindowFullscreen,
    UnavailableWindowSize, UnavailableWindowState, UnavailableWindowStateChanges,
    UnavailableWindowStateRead, UnavailableWindowTitle,
};

use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use anodrel_clipboard::{ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText};
use anodrel_credentials::{CredentialName, CredentialService, CredentialServiceError, Secret};
use anodrel_diagnostics::{DiagnosticsService, DiagnosticsServiceError};
use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
use anodrel_file_access::{
    FileBinaryData, FileBinaryDataError, FileBinaryWriteService, FileBinaryWriteServiceError,
    FileSelectionResult, FileSelectionService, FileSelectionServiceError, FileTextService,
    FileTextServiceError, FileTextWriteService, FileTextWriteServiceError, SaveReference,
    SaveSelectionResult, SaveSelectionService, SaveSelectionServiceError, SelectionReference,
    UnavailableFileBinaryWriteService, UnavailableFileSelectionService, UnavailableFileTextService,
    UnavailableFileTextWriteService, UnavailableSaveSelectionService,
};
use anodrel_file_dialog::{
    FileDialogFilter, FileDialogSelection, FileDialogService, FileDialogServiceError,
};
use anodrel_folder_access::{
    FolderEntryService, FolderEntryServiceError, FolderReference, FolderSelectionResult,
    FolderSelectionService, FolderSelectionServiceError, UnavailableFolderEntryService,
    UnavailableFolderSelectionService,
};
use anodrel_menu::{
    Menu, MenuAction, MenuActionId, MenuModel, MenuService, MenuSession, MenuShortcut, MenuText,
    UnavailableMenuService,
};
use anodrel_network::{NetworkTextService, NetworkTextServiceError, NetworkUrl};
use anodrel_notifications::{
    Notification, NotificationBody, NotificationService, NotificationServiceError,
    NotificationTitle,
};
use anodrel_protocol::{
    Capability, JsonValue, ProtocolErrorCode, ProtocolVersion, RequestEnvelope, ResponseEnvelope,
    is_empty_object, object, sent_at,
};
use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};
use anodrel_ui_session::{
    SessionInteractionCandidate, UiDocumentSession, UiDocumentSnapshot, UiFieldReadError,
    UiFieldReader, UiFieldSnapshot, UiInputMailbox, UiWindowGroup, UiWindowGroupError, UiWindowId,
};
use anodrel_window::{
    WindowFocusService, WindowFocusServiceError, WindowFullscreenMode, WindowFullscreenService,
    WindowFullscreenServiceError, WindowSize, WindowSizeService, WindowSizeServiceError,
    WindowState, WindowStateChangesService, WindowStateChangesServiceError, WindowStateReadService,
    WindowStateReadServiceError, WindowStateService, WindowStateServiceError, WindowTitleProposal,
    WindowTitleService, WindowTitleServiceError,
};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_UI_DOCUMENT_REQUEST_BYTES: usize = 24 * 1024;
pub const MAX_CLIPBOARD_TEXT_REQUEST_BYTES: usize = 24 * 1024;
pub const MAX_EXTERNAL_LINK_REQUEST_BYTES: usize = 2 * 1024;
/// Maximum bytes in the exact `network.fetch_text` URL payload.
pub const MAX_NETWORK_FETCH_REQUEST_BYTES: usize = 2 * 1024;
pub const MAX_FILE_DIALOG_REQUEST_BYTES: usize = 2 * 1024;
pub const MAX_FILE_DIALOG_FILTERS: usize = 8;
pub const MAX_FILE_TEXT_RESPONSE_BYTES: usize = 8 * 1024;
pub const MAX_FILE_TEXT_WRITE_BYTES: usize = 8 * 1024;
/// Maximum encoded JSON bytes in one complete native-menu replacement payload.
pub const MAX_MENU_REPLACE_REQUEST_BYTES: usize = 16 * 1024;
pub const MAX_STORAGE_SNAPSHOT_REQUEST_BYTES: usize = 24 * 1024;

/// The exact external UI document format selected by one protocol operation.
///
/// This stays private to the core dispatcher: applications select only an
/// operation name, while document decoding remains in the portable session
/// layer.
#[derive(Clone, Copy)]
enum UiDocumentFormat {
    V1,
    V2,
    V3,
}

impl UiDocumentFormat {
    const fn document_operation(self) -> &'static str {
        match self {
            Self::V1 => "ui.document.replace",
            Self::V2 => "ui.document.replace.v2",
            Self::V3 => "ui.document.replace.v3",
        }
    }

    const fn window_operation(self) -> &'static str {
        match self {
            Self::V1 => "ui.document.replace.window",
            Self::V2 => "ui.document.replace.window.v2",
            Self::V3 => "ui.document.replace.window.v3",
        }
    }

    const fn open_operation(self) -> &'static str {
        match self {
            Self::V1 => "window.open",
            Self::V2 => "window.open.v2",
            Self::V3 => "window.open.v3",
        }
    }
}

/// One host-created, coalescing request to end an authenticated session.
///
/// This value stores no target, payload, callback, or operating-system state.
/// The native host that supplied it decides which resources to close.
#[derive(Clone, Debug, Default)]
pub struct SessionCloseSignal {
    requested: Arc<AtomicBool>,
}

impl SessionCloseSignal {
    /// Records an idempotent request for the host to end its known session.
    pub fn request(&self) {
        self.requested.store(true, Ordering::Release);
    }

    /// Takes one pending close request, if any.
    pub fn take(&self) -> bool {
        self.requested.swap(false, Ordering::AcqRel)
    }
}

#[derive(Clone, Debug)]
pub struct HostPolicy {
    application_id: String,
    granted_capabilities: Vec<Capability>,
    host_name: String,
}

impl HostPolicy {
    pub fn new(
        application_id: impl Into<String>,
        granted_capabilities: Vec<Capability>,
        host_name: impl Into<String>,
    ) -> Result<Self, &'static str> {
        let application_id = application_id.into();
        let host_name = host_name.into();
        if application_id.trim().is_empty() || host_name.trim().is_empty() {
            return Err("application ID and host name must not be empty");
        }
        if granted_capabilities
            .iter()
            .enumerate()
            .any(|(index, capability)| granted_capabilities[..index].contains(capability))
        {
            return Err("host capability grants must not contain duplicates");
        }
        Ok(Self {
            application_id,
            granted_capabilities,
            host_name,
        })
    }

    fn has(&self, capability: Capability) -> bool {
        self.granted_capabilities.contains(&capability)
    }
}

#[derive(Debug)]
pub struct CoreHost {
    policy: HostPolicy,
    ui_document_session: Option<RefCell<UiDocumentSession>>,
    ui_window_group: Option<UiWindowGroup<WindowTitleProposal>>,
    menu_session: RefCell<MenuSession>,
    ui_input_mailbox: Option<UiInputMailbox>,
    session_close_signal: SessionCloseSignal,
    pending_ui_document_update: Option<RefCell<Option<UiDocumentSnapshot>>>,
    clipboard: Box<dyn ClipboardService>,
    external_links: Box<dyn ExternalLinkService>,
    network: Box<dyn NetworkTextService>,
    notifications: Box<dyn NotificationService>,
    window_title: Box<dyn WindowTitleService>,
    window_state: Box<dyn WindowStateService>,
    window_state_read: Box<dyn WindowStateReadService>,
    window_state_changes: Box<dyn WindowStateChangesService>,
    window_focus: Box<dyn WindowFocusService>,
    window_fullscreen: Box<dyn WindowFullscreenService>,
    window_size: Box<dyn WindowSizeService>,
    menu: Box<dyn MenuService>,
    ui_fields: Box<dyn UiFieldReader>,
    file_dialogs: Box<dyn FileDialogService>,
    folder_selections: Box<dyn FolderSelectionService>,
    folder_entries: Box<dyn FolderEntryService>,
    file_selections: Box<dyn FileSelectionService>,
    file_text: Box<dyn FileTextService>,
    file_save_selections: Box<dyn SaveSelectionService>,
    file_text_write: Box<dyn FileTextWriteService>,
    file_binary_write: Box<dyn FileBinaryWriteService>,
    storage: Box<dyn StorageService>,
    diagnostics: Box<dyn DiagnosticsService>,
    credentials: Box<dyn CredentialService>,
}

fn rfc3339_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = duration.as_secs().min(i64::MAX as u64) as i64;
    let milliseconds = duration.subsec_millis();
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milliseconds:03}Z")
}

// Howard Hinnant's public-domain civil-date conversion, expressed here with
// integer arithmetic so the runtime does not need a time-formatting library.
fn civil_from_days(days_since_unix_epoch: i64) -> (i64, u32, u32) {
    let shifted = days_since_unix_epoch + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests;
