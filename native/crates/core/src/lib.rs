#![forbid(unsafe_code)]

//! Policy-bound handling for one native protocol message.
//!
//! Transports authenticate their sessions before calling this module. Incoming
//! capability context is intentionally ignored: only the host-created policy
//! can authorize a privileged operation.

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
use anodrel_diagnostics::{DiagnosticsService, DiagnosticsServiceError};
use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
use anodrel_file_access::{
    FileSelectionResult, FileSelectionService, FileSelectionServiceError, FileTextService,
    FileTextServiceError, SelectionReference, UnavailableFileSelectionService,
    UnavailableFileTextService,
};
use anodrel_file_dialog::{
    FileDialogFilter, FileDialogSelection, FileDialogService, FileDialogServiceError,
};
use anodrel_protocol::{
    Capability, JsonValue, ProtocolErrorCode, ProtocolVersion, RequestEnvelope, ResponseEnvelope,
    is_empty_object, object, sent_at,
};
use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};
use anodrel_ui_session::{UiDocumentSession, UiDocumentSnapshot, UiInputMailbox};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_UI_DOCUMENT_REQUEST_BYTES: usize = 24 * 1024;
pub const MAX_CLIPBOARD_TEXT_REQUEST_BYTES: usize = 24 * 1024;
pub const MAX_EXTERNAL_LINK_REQUEST_BYTES: usize = 2 * 1024;
pub const MAX_FILE_DIALOG_REQUEST_BYTES: usize = 2 * 1024;
pub const MAX_FILE_DIALOG_FILTERS: usize = 8;
pub const MAX_FILE_TEXT_RESPONSE_BYTES: usize = 8 * 1024;
pub const MAX_STORAGE_SNAPSHOT_REQUEST_BYTES: usize = 24 * 1024;

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
    ui_document_session: RefCell<UiDocumentSession>,
    ui_input_mailbox: UiInputMailbox,
    session_close_signal: SessionCloseSignal,
    pending_ui_document_update: RefCell<Option<UiDocumentSnapshot>>,
    clipboard: Box<dyn ClipboardService>,
    external_links: Box<dyn ExternalLinkService>,
    file_dialogs: Box<dyn FileDialogService>,
    file_selections: Box<dyn FileSelectionService>,
    file_text: Box<dyn FileTextService>,
    storage: Box<dyn StorageService>,
    diagnostics: Box<dyn DiagnosticsService>,
}

#[derive(Debug)]
struct UnavailableClipboard;

impl ClipboardService for UnavailableClipboard {
    fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }

    fn write_text(&self, _text: &ClipboardText) -> Result<(), ClipboardServiceError> {
        Err(ClipboardServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableExternalLinks;

impl ExternalLinkService for UnavailableExternalLinks {
    fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        Err(ExternalLinkOpenError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableFileDialogs;

impl FileDialogService for UnavailableFileDialogs {
    fn open_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }
}

#[derive(Debug)]
struct UnavailableStorage;

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
struct UnavailableDiagnostics;

impl DiagnosticsService for UnavailableDiagnostics {
    fn entries(&self) -> Result<Vec<anodrel_diagnostics::Entry>, DiagnosticsServiceError> {
        Err(DiagnosticsServiceError::Unavailable)
    }
}

impl CoreHost {
    pub fn new(policy: HostPolicy) -> Self {
        Self::with_session_components(policy, UiInputMailbox::new(), SessionCloseSignal::default())
    }

    /// Creates a host core that validates semantic input from one supplied
    /// per-session mailbox.
    pub fn with_ui_input_mailbox(policy: HostPolicy, ui_input_mailbox: UiInputMailbox) -> Self {
        Self::with_session_components(policy, ui_input_mailbox, SessionCloseSignal::default())
    }

    /// Creates a host core with explicit native input and session-close signals.
    pub fn with_session_components(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
    ) -> Self {
        Self::with_session_components_and_all_services(
            policy,
            ui_input_mailbox,
            session_close_signal,
            UnavailableClipboard,
            UnavailableExternalLinks,
            UnavailableFileDialogs,
        )
    }

    /// Creates a host core with explicit native components and one injected
    /// portable clipboard service.
    pub fn with_session_components_and_clipboard(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services(
            policy,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            UnavailableExternalLinks,
            UnavailableFileDialogs,
        )
    }

    /// Creates a host core with explicit native components and injected
    /// portable clipboard and external-link services.
    pub fn with_session_components_and_services(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services(
            policy,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            UnavailableFileDialogs,
        )
    }

    /// Creates a host core with all currently supported injected platform
    /// services. Dialog implementations must route native UI through the host
    /// UI thread rather than invoking an OS dialog from this core.
    pub fn with_session_components_and_all_services(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access(
            policy,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            UnavailableFileSelectionService,
            UnavailableFileTextService,
        )
    }

    /// Creates a host core with all injected platform services, including the
    /// separate selection-capture and selected-file text boundaries.
    ///
    /// The selection service must bind a picker choice to retained native
    /// identity before returning success. The text service consumes only that
    /// retained state, never a request path.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage(
            policy,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            file_selections,
            file_text,
            UnavailableStorage,
        )
    }

    /// Creates a host core with all injected services, including one
    /// host-selected application-state store for this authenticated session.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
        storage: impl StorageService + 'static,
    ) -> Self {
        Self::with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
            policy,
            ui_input_mailbox,
            session_close_signal,
            clipboard,
            external_links,
            file_dialogs,
            file_selections,
            file_text,
            storage,
            UnavailableDiagnostics,
        )
    }

    /// Creates a host core with the bounded host-owned diagnostic source for
    /// this authenticated session.
    #[allow(clippy::too_many_arguments)]
    pub fn with_session_components_and_all_services_and_file_access_and_storage_and_diagnostics(
        policy: HostPolicy,
        ui_input_mailbox: UiInputMailbox,
        session_close_signal: SessionCloseSignal,
        clipboard: impl ClipboardService + 'static,
        external_links: impl ExternalLinkService + 'static,
        file_dialogs: impl FileDialogService + 'static,
        file_selections: impl FileSelectionService + 'static,
        file_text: impl FileTextService + 'static,
        storage: impl StorageService + 'static,
        diagnostics: impl DiagnosticsService + 'static,
    ) -> Self {
        Self {
            policy,
            ui_document_session: RefCell::new(UiDocumentSession::new()),
            ui_input_mailbox,
            session_close_signal,
            pending_ui_document_update: RefCell::new(None),
            clipboard: Box::new(clipboard),
            external_links: Box::new(external_links),
            file_dialogs: Box::new(file_dialogs),
            file_selections: Box::new(file_selections),
            file_text: Box::new(file_text),
            storage: Box::new(storage),
            diagnostics: Box::new(diagnostics),
        }
    }

    /// Takes the latest accepted document snapshot not yet observed by the
    /// transport that owns this core host.
    pub fn take_ui_document_update(&self) -> Option<UiDocumentSnapshot> {
        self.pending_ui_document_update.borrow_mut().take()
    }

    pub fn handle_json(&self, message: &str) -> String {
        let response = if message.len() > MAX_REQUEST_BYTES {
            self.failure(
                "invalid-request".to_owned(),
                ProtocolErrorCode::RequestInvalid,
                "Request exceeded the native transport message-size limit.",
                None,
            )
        } else {
            match JsonValue::parse(message) {
                Ok(value) => match RequestEnvelope::from_json(value) {
                    Ok(request) => self.handle(request),
                    Err(_) => self.failure(
                        "invalid-request".to_owned(),
                        ProtocolErrorCode::RequestInvalid,
                        "Request envelope is malformed.",
                        None,
                    ),
                },
                Err(_) => self.failure(
                    "invalid-request".to_owned(),
                    ProtocolErrorCode::RequestInvalid,
                    "Request envelope is malformed.",
                    None,
                ),
            }
        };
        response.to_json()
    }

    fn handle(&self, request: RequestEnvelope) -> JsonValue {
        if !request.protocol_version.is_supported() {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::ProtocolVersionUnsupported,
                format!(
                    "Protocol {}.{} is not supported.",
                    request.protocol_version.major, request.protocol_version.minor
                ),
                None,
            );
        }

        match request.operation.as_str() {
            "platform.ping" => self.handle_ping(request),
            "platform.capabilities" => self.handle_capabilities(request),
            "platform.health" => self.handle_health(request),
            "diagnostics.entries.read" if request.protocol_version.minor >= 11 => {
                self.handle_diagnostics_entries_read(request)
            }
            "ui.document.replace" if request.protocol_version.minor >= 1 => {
                self.handle_ui_document_replace(request, false)
            }
            "ui.document.replace.v2" if request.protocol_version.minor >= 4 => {
                self.handle_ui_document_replace(request, true)
            }
            "ui.events.read" if request.protocol_version.minor >= 2 => {
                self.handle_ui_events_read(request)
            }
            "session.close" if request.protocol_version.minor >= 3 => {
                self.handle_session_close(request)
            }
            "clipboard.read" if request.protocol_version.minor >= 5 => {
                self.handle_clipboard_read(request)
            }
            "clipboard.write" if request.protocol_version.minor >= 5 => {
                self.handle_clipboard_write(request)
            }
            "external.open" if request.protocol_version.minor >= 6 => {
                self.handle_external_open(request)
            }
            "dialog.open_file" if request.protocol_version.minor >= 7 => {
                self.handle_file_dialog_open(request)
            }
            "dialog.save_file" if request.protocol_version.minor >= 8 => {
                self.handle_file_dialog_save(request)
            }
            "dialog.open_file.v2" if request.protocol_version.minor >= 9 => {
                self.handle_file_dialog_open_with_reference(request)
            }
            "file.read_text" if request.protocol_version.minor >= 9 => {
                self.handle_file_text_read(request)
            }
            "storage.state.read" if request.protocol_version.minor >= 10 => {
                self.handle_storage_read(request)
            }
            "storage.state.replace" if request.protocol_version.minor >= 10 => {
                self.handle_storage_replace(request)
            }
            "storage.state.clear" if request.protocol_version.minor >= 10 => {
                self.handle_storage_clear(request)
            }
            _ => self.failure(
                request.request_id,
                ProtocolErrorCode::OperationUnsupported,
                format!(
                    "Operation {} is not supported by this host.",
                    request.operation
                ),
                None,
            ),
        }
    }

    fn handle_ping(&self, request: RequestEnvelope) -> JsonValue {
        if sent_at(&request.payload).is_none() {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "platform.ping requires a sentAt string.",
                None,
            );
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                ("receivedAt", JsonValue::String(rfc3339_now())),
                ("hostName", JsonValue::String(self.policy.host_name.clone())),
            ]),
        )
    }

    fn handle_capabilities(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "platform.capabilities does not accept a payload.",
                None,
            );
        }
        let capabilities = self
            .policy
            .granted_capabilities
            .iter()
            .map(|capability| JsonValue::String(capability.as_str().to_owned()))
            .collect();
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                (
                    "applicationId",
                    JsonValue::String(self.policy.application_id.clone()),
                ),
                ("grantedCapabilities", JsonValue::Array(capabilities)),
            ]),
        )
    }

    fn handle_health(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "platform.health does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::DiagnosticsRead) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                "platform.health requires the diagnostics.read capability.",
                Some(BTreeMap::from([(
                    "capability".to_owned(),
                    JsonValue::String("diagnostics.read".to_owned()),
                )])),
            );
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                ("status", JsonValue::String("ready".to_owned())),
                ("hostName", JsonValue::String(self.policy.host_name.clone())),
                ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
            ]),
        )
    }

    fn handle_diagnostics_entries_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "diagnostics.entries.read does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::DiagnosticsRead) {
            return self.capability_denied(request.request_id, "diagnostics.read");
        }
        match self.diagnostics.entries() {
            Ok(entries) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([(
                    "entries",
                    JsonValue::Array(
                        entries
                            .into_iter()
                            .map(|entry| {
                                object([
                                    ("sequence", JsonValue::String(entry.sequence().to_string())),
                                    ("level", JsonValue::String(entry.level().label().to_owned())),
                                    ("component", JsonValue::String(entry.component().to_owned())),
                                    ("event", JsonValue::String(entry.message().to_owned())),
                                ])
                            })
                            .collect(),
                    ),
                )]),
            ),
            Err(DiagnosticsServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DiagnosticsUnavailable,
                "diagnostic entries are unavailable.",
                None,
            ),
        }
    }

    fn handle_ui_document_replace(&self, request: RequestEnvelope, version_two: bool) -> JsonValue {
        let operation = if version_two {
            "ui.document.replace.v2"
        } else {
            "ui.document.replace"
        };
        let Some(document) = ui_document_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} requires one document string."),
                None,
            );
        };
        if !self.policy.has(Capability::UiDocumentWrite) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                format!("{operation} requires the ui.document.write capability."),
                Some(BTreeMap::from([(
                    "capability".to_owned(),
                    JsonValue::String("ui.document.write".to_owned()),
                )])),
            );
        }
        if document.len() > MAX_UI_DOCUMENT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document exceeded the operation size limit."),
                None,
            );
        }

        let Some(snapshot) = ({
            let mut session = self.ui_document_session.borrow_mut();
            let revision = if version_two {
                session.replace_document_v2(document)
            } else {
                session.replace_document(document)
            };
            revision.ok().and_then(|revision| {
                session
                    .snapshot()
                    .filter(|snapshot| snapshot.revision() == revision)
            })
        }) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document is invalid."),
                None,
            );
        };
        let revision = snapshot.revision();
        *self.pending_ui_document_update.borrow_mut() = Some(snapshot);
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([("revision", JsonValue::String(revision.value().to_string()))]),
        )
    }

    fn handle_ui_events_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "ui.events.read does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::UiEventsRead) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                "ui.events.read requires the ui.events.read capability.",
                Some(BTreeMap::from([(
                    "capability".to_owned(),
                    JsonValue::String("ui.events.read".to_owned()),
                )])),
            );
        }

        let batch = self.ui_input_mailbox.drain();
        let dropped = batch.dropped();
        let mut discarded = 0_u32;
        let mut events = Vec::new();
        for candidate in batch.into_candidates() {
            let (revision, event) = candidate.into_parts();
            match self
                .ui_document_session
                .borrow()
                .accept_event(revision, event)
            {
                Ok(event) => events.push(ui_action_event(event)),
                Err(_) => discarded = discarded.saturating_add(1),
            }
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([
                ("events", JsonValue::Array(events)),
                ("dropped", JsonValue::Number(dropped.to_string())),
                ("discarded", JsonValue::Number(discarded.to_string())),
            ]),
        )
    }

    fn handle_session_close(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "session.close does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::SessionClose) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                "session.close requires the session.close capability.",
                Some(BTreeMap::from([(
                    "capability".to_owned(),
                    JsonValue::String("session.close".to_owned()),
                )])),
            );
        }
        self.session_close_signal.request();
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([("status", JsonValue::String("accepted".to_owned()))]),
        )
    }

    fn handle_clipboard_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "clipboard.read does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::ClipboardRead) {
            return self.capability_denied(request.request_id, "clipboard.read");
        }
        match self.clipboard.read_text() {
            Ok(ClipboardRead::Text(text)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("text".to_owned())),
                    ("text", JsonValue::String(text.as_str().to_owned())),
                ]),
            ),
            Ok(ClipboardRead::NoText) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("no_text".to_owned()))]),
            ),
            Err(error) => self.clipboard_failure(request.request_id, error),
        }
    }

    fn handle_clipboard_write(&self, request: RequestEnvelope) -> JsonValue {
        let Some(text) = clipboard_write_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "clipboard.write requires one bounded text string.",
                None,
            );
        };
        if text.len() > MAX_CLIPBOARD_TEXT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "clipboard.write text exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::ClipboardWrite) {
            return self.capability_denied(request.request_id, "clipboard.write");
        }
        let text = match ClipboardText::new(text) {
            Ok(text) => text,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "clipboard.write text exceeded the portable size limit.",
                    None,
                );
            }
        };
        match self.clipboard.write_text(&text) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("written".to_owned()))]),
            ),
            Err(error) => self.clipboard_failure(request.request_id, error),
        }
    }

    fn handle_external_open(&self, request: RequestEnvelope) -> JsonValue {
        let Some(url) = external_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "external.open requires one bounded URL string.",
                None,
            );
        };
        if url.len() > MAX_EXTERNAL_LINK_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "external.open URL exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::ExternalOpen) {
            return self.capability_denied(request.request_id, "external.open");
        }
        let link = match ExternalLink::parse(url) {
            Ok(link) => link,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "external.open URL is invalid.",
                    None,
                );
            }
        };
        match self.external_links.open(&link) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("opened".to_owned()))]),
            ),
            Err(ExternalLinkOpenError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::ExternalUnavailable,
                "external link handler is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_dialog_open(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogOpenFile) {
            return self.capability_denied(request.request_id, "dialog.open_file");
        }
        match self.file_dialogs.open_file(&filters) {
            Ok(FileDialogSelection::Selected(path)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("selected".to_owned())),
                    (
                        "path",
                        JsonValue::String(path.as_path().to_string_lossy().into_owned()),
                    ),
                ]),
            ),
            Ok(FileDialogSelection::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            // The typed mailbox prevents this, but an injected host service
            // must never turn a save destination into an open-file result.
            Ok(FileDialogSelection::Saved(_)) | Ok(FileDialogSelection::Captured(_, _)) => self
                .failure(
                    request.request_id,
                    ProtocolErrorCode::DialogUnavailable,
                    "file dialog returned an incompatible result.",
                    None,
                ),
            Err(FileDialogServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_dialog_save(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogSaveFile) {
            return self.capability_denied(request.request_id, "dialog.save_file");
        }
        match self.file_dialogs.save_file(&filters) {
            Ok(FileDialogSelection::Saved(path)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("saved".to_owned())),
                    (
                        "path",
                        JsonValue::String(path.as_path().to_string_lossy().into_owned()),
                    ),
                ]),
            ),
            Ok(FileDialogSelection::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Captured(_, _))
            | Err(FileDialogServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_dialog_open_with_reference(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file.v2 requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file.v2 filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogOpenFile) {
            return self.capability_denied(request.request_id, "dialog.open_file");
        }
        match self.file_selections.open_file(&filters) {
            Ok(FileSelectionResult::Selected(selection)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("selected".to_owned())),
                    (
                        "path",
                        JsonValue::String(
                            selection.path().as_path().to_string_lossy().into_owned(),
                        ),
                    ),
                    (
                        "selectionReference",
                        JsonValue::String(selection.reference().as_str().to_owned()),
                    ),
                ]),
            ),
            Ok(FileSelectionResult::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Err(FileSelectionServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    fn handle_file_text_read(&self, request: RequestEnvelope) -> JsonValue {
        let Some(reference) = file_text_read_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "file.read_text requires one exact selection reference.",
                None,
            );
        };
        if !self.policy.has(Capability::FileReadText) {
            return self.capability_denied(request.request_id, "file.read_text");
        }
        match self.file_text.read_text(&reference) {
            Ok(text) if text.len() <= MAX_FILE_TEXT_RESPONSE_BYTES => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("text".to_owned())),
                    ("text", JsonValue::String(text)),
                ]),
            ),
            Ok(_) | Err(FileTextServiceError::TooLarge) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextTooLarge,
                "selected file text is too large.",
                None,
            ),
            Err(FileTextServiceError::InvalidText) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextInvalid,
                "selected file text is invalid.",
                None,
            ),
            Err(FileTextServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileUnavailable,
                "selected file is unavailable.",
                None,
            ),
        }
    }

    fn handle_storage_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.read requires an empty payload.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateRead) {
            return self.capability_denied(request.request_id, "storage.state.read");
        }
        match self.storage.read() {
            Ok(StorageRead::Absent) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("absent".to_owned()))]),
            ),
            Ok(StorageRead::Snapshot(snapshot))
                if snapshot.as_str().len() <= MAX_STORAGE_SNAPSHOT_REQUEST_BYTES =>
            {
                ResponseEnvelope::success(
                    request.request_id,
                    &self.policy.host_name,
                    object([
                        ("status", JsonValue::String("snapshot".to_owned())),
                        ("snapshot", JsonValue::String(snapshot.as_str().to_owned())),
                    ]),
                )
            }
            Ok(StorageRead::Snapshot(_)) | Err(StorageServiceError::StoredSnapshotTooLarge) => self
                .storage_failure(
                    request.request_id,
                    StorageServiceError::StoredSnapshotTooLarge,
                ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    fn handle_storage_replace(&self, request: RequestEnvelope) -> JsonValue {
        let Some(snapshot) = storage_replace_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.replace requires one exact snapshot.",
                None,
            );
        };
        if snapshot.len() > MAX_STORAGE_SNAPSHOT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage snapshot is too large.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateReplace) {
            return self.capability_denied(request.request_id, "storage.state.replace");
        }
        let snapshot = match StorageSnapshot::new(snapshot.to_owned()) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "storage snapshot is too large.",
                    None,
                );
            }
        };
        match self.storage.replace(&snapshot) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("replaced".to_owned()))]),
            ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    fn handle_storage_clear(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.clear requires an empty payload.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateClear) {
            return self.capability_denied(request.request_id, "storage.state.clear");
        }
        match self.storage.clear() {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cleared".to_owned()))]),
            ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    fn capability_denied(&self, request_id: String, capability: &str) -> JsonValue {
        self.failure(
            request_id,
            ProtocolErrorCode::CapabilityDenied,
            format!("operation requires the {capability} capability."),
            Some(BTreeMap::from([(
                "capability".to_owned(),
                JsonValue::String(capability.to_owned()),
            )])),
        )
    }

    fn clipboard_failure(&self, request_id: String, error: ClipboardServiceError) -> JsonValue {
        let (code, message) = match error {
            ClipboardServiceError::Unavailable => (
                ProtocolErrorCode::ClipboardUnavailable,
                "clipboard is unavailable.",
            ),
            ClipboardServiceError::StoredTextInvalid => (
                ProtocolErrorCode::ClipboardTextInvalid,
                "clipboard text is invalid.",
            ),
            ClipboardServiceError::StoredTextTooLarge => (
                ProtocolErrorCode::ClipboardTextTooLarge,
                "clipboard text is too large.",
            ),
        };
        self.failure(request_id, code, message, None)
    }

    fn storage_failure(&self, request_id: String, error: StorageServiceError) -> JsonValue {
        let (code, message) = match error {
            StorageServiceError::Unavailable => (
                ProtocolErrorCode::StorageUnavailable,
                "application state is unavailable.",
            ),
            StorageServiceError::StoredSnapshotInvalid => (
                ProtocolErrorCode::StorageSnapshotInvalid,
                "stored application state is invalid.",
            ),
            StorageServiceError::StoredSnapshotTooLarge => (
                ProtocolErrorCode::StorageSnapshotTooLarge,
                "stored application state is too large.",
            ),
        };
        self.failure(request_id, code, message, None)
    }

    fn failure(
        &self,
        request_id: String,
        code: ProtocolErrorCode,
        message: impl Into<String>,
        details: Option<BTreeMap<String, JsonValue>>,
    ) -> JsonValue {
        ResponseEnvelope::failure(request_id, &self.policy.host_name, code, message, details)
    }
}

fn ui_action_event(event: anodrel_ui_session::UiApplicationEvent) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("ui.action.invoked".to_owned()),
        ),
        ("source", JsonValue::String("native.ui".to_owned())),
        (
            "schemaVersion",
            ProtocolVersion { major: 1, minor: 0 }.to_json(),
        ),
        (
            "payload",
            object([
                (
                    "revision",
                    JsonValue::String(event.revision().value().to_string()),
                ),
                (
                    "action",
                    JsonValue::String(event.action().as_str().to_owned()),
                ),
            ]),
        ),
    ])
}

fn ui_document_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("document"))
        .flatten()
        .and_then(JsonValue::as_string)
}

fn clipboard_write_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("text"))
        .flatten()
        .and_then(JsonValue::as_string)
}

fn external_open_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("url"))
        .flatten()
        .and_then(JsonValue::as_string)
}

fn file_dialog_open_payload(value: &JsonValue) -> Option<Vec<FileDialogFilter>> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    let JsonValue::Array(filters) = fields.get("filters")? else {
        return None;
    };
    if filters.is_empty() || filters.len() > MAX_FILE_DIALOG_FILTERS {
        return None;
    }
    filters
        .iter()
        .map(|filter| {
            let fields = filter.as_object()?;
            if fields.len() != 2 {
                return None;
            }
            let label = fields.get("label")?.as_string()?.to_owned();
            let JsonValue::Array(extensions) = fields.get("extensions")? else {
                return None;
            };
            let extensions = extensions
                .iter()
                .map(|extension| extension.as_string().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?;
            FileDialogFilter::new(label, extensions).ok()
        })
        .collect()
}

fn file_text_read_payload(value: &JsonValue) -> Option<SelectionReference> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    SelectionReference::new(fields.get("selectionReference")?.as_string()?.to_owned()).ok()
}

fn storage_replace_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("snapshot"))
        .flatten()
        .and_then(JsonValue::as_string)
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
mod tests {
    use std::cell::RefCell;
    use std::sync::Mutex;

    use anodrel_clipboard::{
        ClipboardRead, ClipboardService, ClipboardServiceError, ClipboardText,
    };
    use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};
    use anodrel_file_access::{FileSelection, FileSelectionService, FileTextService};
    use anodrel_file_dialog::{SaveFilePath, SelectedFilePath};
    use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};
    use anodrel_ui::{ElementId, UiEvent};
    use anodrel_ui_session::UiInputCandidate;

    use super::*;

    fn host(grants: Vec<Capability>) -> CoreHost {
        CoreHost::new(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        )
    }

    #[derive(Debug)]
    struct MemoryClipboard {
        text: RefCell<Option<ClipboardText>>,
    }

    impl MemoryClipboard {
        fn with_text(text: Option<&str>) -> Self {
            Self {
                text: RefCell::new(
                    text.map(|value| ClipboardText::new(value).expect("fixture text")),
                ),
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
    struct FailingClipboard(ClipboardServiceError);

    impl ClipboardService for FailingClipboard {
        fn read_text(&self) -> Result<ClipboardRead, ClipboardServiceError> {
            Err(self.0)
        }

        fn write_text(&self, _text: &ClipboardText) -> Result<(), ClipboardServiceError> {
            Err(self.0)
        }
    }

    #[derive(Debug, Default)]
    struct RecordingExternalLinks(RefCell<Option<ExternalLink>>);

    impl ExternalLinkService for RecordingExternalLinks {
        fn open(&self, link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
            *self.0.borrow_mut() = Some(link.clone());
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingExternalLinks;

    impl ExternalLinkService for FailingExternalLinks {
        fn open(&self, _link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
            Err(ExternalLinkOpenError::Unavailable)
        }
    }

    #[derive(Debug)]
    struct CancellingFileDialog;

    impl FileDialogService for CancellingFileDialog {
        fn open_file(
            &self,
            _filters: &[FileDialogFilter],
        ) -> Result<FileDialogSelection, FileDialogServiceError> {
            Ok(FileDialogSelection::Cancelled)
        }
    }

    #[derive(Debug)]
    struct SavingFileDialog;

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
    struct CapturingFileDialog;

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
    struct FixedFileText(Result<String, FileTextServiceError>);

    impl FileTextService for FixedFileText {
        fn read_text(
            &self,
            _reference: &SelectionReference,
        ) -> Result<String, FileTextServiceError> {
            self.0.clone()
        }
    }

    #[derive(Debug)]
    struct MemoryStorage(Mutex<Result<StorageRead, StorageServiceError>>);

    impl MemoryStorage {
        fn with_state(state: StorageRead) -> Self {
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

    fn clipboard_host(
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

    fn external_host(
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

    fn file_dialog_host(
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

    fn file_access_host(
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

    fn storage_host(grants: Vec<Capability>, storage: impl StorageService + 'static) -> CoreHost {
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

    fn request(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":0}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_1(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":1}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_2(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":2}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_3(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":3}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_4(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":4}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_5(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":5}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_6(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":6}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_7(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":7}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_8(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":8}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_9(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":9}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn request_v1_10(operation: &str, payload: &str) -> String {
        format!(
            r#"{{"protocolVersion":{{"major":1,"minor":10}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
        )
    }

    fn ui_document_payload(document: &str) -> String {
        object([("document", JsonValue::String(document.to_owned()))]).to_json()
    }

    fn valid_ui_document(label: &str) -> String {
        format!(
            r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"root","kind":"action","label":"{label}","fontSize":16,"enabled":true,"tone":"accent"}}}}"#
        )
    }

    fn valid_ui_document_v2() -> &'static str {
        r#"{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"content","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}}"#
    }

    fn field<'a>(value: &'a JsonValue, field: &str) -> &'a JsonValue {
        &value.as_object().expect("response is an object")[field]
    }

    #[test]
    fn accepts_ping_and_formats_a_utc_timestamp() {
        let response = JsonValue::parse(&host(vec![]).handle_json(&request(
            "platform.ping",
            r#"{"sentAt":"2026-07-31T12:00:00.000Z"}"#,
        )))
        .expect("response JSON is valid");
        assert_eq!(field(&response, "status").as_string(), Some("success"));
        assert!(
            field(field(&response, "result"), "receivedAt")
                .as_string()
                .is_some_and(|timestamp| timestamp.ends_with('Z'))
        );
    }

    #[test]
    fn rejects_forged_capability_context() {
        let response = JsonValue::parse(&host(vec![]).handle_json(&format!(
            r#"{},"capabilityContext":{{"grantedCapabilities":["diagnostics.read"]}}}}"#,
            request("platform.health", "{}")
                .strip_suffix('}')
                .expect("request ends with a brace")
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("capability.denied")
        );
    }

    #[test]
    fn replaces_ui_documents_only_with_the_current_capability_and_protocol_minor() {
        let document = valid_ui_document("Continue");
        let update_request = request_v1_1("ui.document.replace", &ui_document_payload(&document));

        let denied = JsonValue::parse(&host(vec![]).handle_json(&update_request))
            .expect("response JSON is valid");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let host = host(vec![Capability::UiDocumentWrite]);
        let first =
            JsonValue::parse(&host.handle_json(&update_request)).expect("response JSON is valid");
        assert_eq!(field(&first, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&first, "result"), "revision").as_string(),
            Some("1")
        );
        let first_snapshot = host
            .take_ui_document_update()
            .expect("accepted document is available to the transport");
        assert_eq!(first_snapshot.revision().value(), 1);
        assert_eq!(first_snapshot.document().root().id().as_str(), "root");
        assert!(host.take_ui_document_update().is_none());

        let invalid = request_v1_1("ui.document.replace", &ui_document_payload("not JSON"));
        let invalid =
            JsonValue::parse(&host.handle_json(&invalid)).expect("response JSON is valid");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let second_document = valid_ui_document("Continue safely");
        let second = JsonValue::parse(&host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&second_document),
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&second, "result"), "revision").as_string(),
            Some("2")
        );

        let old_minor = JsonValue::parse(&host.handle_json(&request(
            "ui.document.replace",
            &ui_document_payload(&document),
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&old_minor, "error"), "code").as_string(),
            Some("operation.unsupported")
        );

        let oversized = request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&"x".repeat(MAX_UI_DOCUMENT_REQUEST_BYTES + 1)),
        );
        let oversized =
            JsonValue::parse(&host.handle_json(&oversized)).expect("response JSON is valid");
        assert_eq!(
            field(field(&oversized, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn replaces_version_two_documents_only_through_the_new_operation() {
        let host = host(vec![Capability::UiDocumentWrite]);
        let document = valid_ui_document_v2();

        let accepted = JsonValue::parse(&host.handle_json(&request_v1_4(
            "ui.document.replace.v2",
            &ui_document_payload(document),
        )))
        .expect("response JSON is valid");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        let snapshot = host
            .take_ui_document_update()
            .expect("accepted version two document is delivered");
        assert_eq!(snapshot.document().root().id().as_str(), "viewport");

        let wrong_operation = JsonValue::parse(&host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(document),
        )))
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&wrong_operation, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn reads_only_current_enabled_ui_actions_from_the_supplied_input_mailbox() {
        let mailbox = UiInputMailbox::new();
        let host = CoreHost::with_ui_input_mailbox(
            HostPolicy::new(
                "test.application",
                vec![Capability::UiDocumentWrite, Capability::UiEventsRead],
                "test-host",
            )
            .expect("test policy is valid"),
            mailbox.clone(),
        );
        let document = valid_ui_document("Continue");
        let update = JsonValue::parse(&host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&document),
        )))
        .expect("update response is JSON");
        assert_eq!(
            field(field(&update, "result"), "revision").as_string(),
            Some("1")
        );

        let current = host
            .take_ui_document_update()
            .expect("accepted document is available")
            .revision();
        let action = UiEvent::ActionInvoked(ElementId::new("root").expect("test ID is valid"));
        mailbox.push(UiInputCandidate::new(current, action.clone()));
        let read = JsonValue::parse(&host.handle_json(&request_v1_2("ui.events.read", "{}")))
            .expect("event response is JSON");
        let result = field(&read, "result");
        assert_eq!(field(result, "dropped"), &JsonValue::Number("0".to_owned()));
        assert_eq!(
            field(result, "discarded"),
            &JsonValue::Number("0".to_owned())
        );
        let JsonValue::Array(events) = field(result, "events") else {
            panic!("events is an array");
        };
        assert_eq!(events.len(), 1);
        assert_eq!(
            field(&events[0], "eventName").as_string(),
            Some("ui.action.invoked")
        );
        assert_eq!(
            field(field(&events[0], "payload"), "action").as_string(),
            Some("root")
        );

        let replacement = valid_ui_document("Continue safely");
        let _ = host.handle_json(&request_v1_1(
            "ui.document.replace",
            &ui_document_payload(&replacement),
        ));
        mailbox.push(UiInputCandidate::new(current, action));
        let stale = JsonValue::parse(&host.handle_json(&request_v1_2("ui.events.read", "{}")))
            .expect("stale event response is JSON");
        let JsonValue::Array(events) = field(field(&stale, "result"), "events") else {
            panic!("events is an array");
        };
        assert!(events.is_empty());
        assert_eq!(
            field(field(&stale, "result"), "discarded"),
            &JsonValue::Number("1".to_owned())
        );
    }

    #[test]
    fn accepts_only_a_granted_current_protocol_session_close_request() {
        let signal = SessionCloseSignal::default();
        let close_host = CoreHost::with_session_components(
            HostPolicy::new(
                "test.application",
                vec![Capability::SessionClose],
                "test-host",
            )
            .expect("test policy is valid"),
            UiInputMailbox::new(),
            signal.clone(),
        );
        let accepted =
            JsonValue::parse(&close_host.handle_json(&request_v1_3("session.close", "{}")))
                .expect("response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("accepted")
        );
        assert!(signal.take());
        assert!(!signal.take());

        let denied =
            JsonValue::parse(&host(vec![]).handle_json(&request_v1_3("session.close", "{}")))
                .expect("response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let old_minor =
            JsonValue::parse(&close_host.handle_json(&request_v1_2("session.close", "{}")))
                .expect("response is JSON");
        assert_eq!(
            field(field(&old_minor, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn clipboard_operations_are_separate_bounded_and_capability_checked() {
        let clipboard_host = clipboard_host(
            vec![Capability::ClipboardRead, Capability::ClipboardWrite],
            MemoryClipboard::with_text(Some("before")),
        );

        let read =
            JsonValue::parse(&clipboard_host.handle_json(&request_v1_5("clipboard.read", "{}")))
                .expect("clipboard read response is JSON");
        assert_eq!(field(&read, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&read, "result"), "status").as_string(),
            Some("text")
        );
        assert_eq!(
            field(field(&read, "result"), "text").as_string(),
            Some("before")
        );

        let write = JsonValue::parse(
            &clipboard_host.handle_json(&request_v1_5("clipboard.write", r#"{"text":"after"}"#)),
        )
        .expect("clipboard write response is JSON");
        assert_eq!(
            field(field(&write, "result"), "status").as_string(),
            Some("written")
        );

        let updated =
            JsonValue::parse(&clipboard_host.handle_json(&request_v1_5("clipboard.read", "{}")))
                .expect("updated clipboard read response is JSON");
        assert_eq!(
            field(field(&updated, "result"), "text").as_string(),
            Some("after")
        );

        let denied =
            JsonValue::parse(&host(vec![]).handle_json(&request_v1_5("clipboard.read", "{}")))
                .expect("denied clipboard response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let oversized = object([(
            "text",
            JsonValue::String("x".repeat(MAX_CLIPBOARD_TEXT_REQUEST_BYTES + 1)),
        )])
        .to_json();
        let rejected = JsonValue::parse(
            &clipboard_host.handle_json(&request_v1_5("clipboard.write", &oversized)),
        )
        .expect("oversized clipboard response is JSON");
        assert_eq!(
            field(field(&rejected, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn clipboard_service_failures_have_safe_stable_protocol_codes() {
        let host = clipboard_host(
            vec![Capability::ClipboardRead, Capability::ClipboardWrite],
            FailingClipboard(ClipboardServiceError::StoredTextInvalid),
        );
        let response = JsonValue::parse(&host.handle_json(&request_v1_5("clipboard.read", "{}")))
            .expect("clipboard failure response is JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("clipboard.text_invalid")
        );
        assert!(
            field(field(&response, "error"), "message")
                .as_string()
                .is_some_and(|message| !message.contains("before"))
        );
    }

    #[test]
    fn external_open_requires_its_own_grant_and_validated_https_url() {
        let external_host = external_host(
            vec![Capability::ExternalOpen],
            RecordingExternalLinks::default(),
        );
        let accepted = JsonValue::parse(&external_host.handle_json(&request_v1_6(
            "external.open",
            r#"{"url":"https://docs.anodrel.dev/guide"}"#,
        )))
        .expect("external open response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("opened")
        );

        let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_6(
            "external.open",
            r#"{"url":"https://docs.anodrel.dev/guide"}"#,
        )))
        .expect("denied external open response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&external_host.handle_json(&request_v1_6(
            "external.open",
            r#"{"url":"file:///C:/private.txt"}"#,
        )))
        .expect("invalid external open response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn external_service_failure_never_exposes_a_url_or_native_status() {
        let host = external_host(vec![Capability::ExternalOpen], FailingExternalLinks);
        let response = JsonValue::parse(&host.handle_json(&request_v1_6(
            "external.open",
            r#"{"url":"https://docs.anodrel.dev/private"}"#,
        )))
        .expect("external failure response is JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("external.unavailable")
        );
        assert!(
            field(field(&response, "error"), "message")
                .as_string()
                .is_some_and(|message| !message.contains("private"))
        );
    }

    #[test]
    fn file_dialog_requires_its_own_grant_and_returns_only_cancellation_or_a_path() {
        let accepted_host =
            file_dialog_host(vec![Capability::DialogOpenFile], CancellingFileDialog);
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
            "dialog.open_file",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("dialog response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("cancelled")
        );

        let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_7(
            "dialog.open_file",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("denied dialog response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
            "dialog.open_file",
            r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
        )))
        .expect("invalid dialog response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn save_dialog_requires_its_own_grant_and_returns_only_cancellation_or_a_destination() {
        let accepted_host = file_dialog_host(vec![Capability::DialogSaveFile], SavingFileDialog);
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_8(
            "dialog.save_file",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("save dialog response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "status").as_string(),
            Some("saved")
        );
        assert!(
            field(field(&accepted, "result"), "path")
                .as_string()
                .is_some_and(|path| !path.is_empty())
        );

        let denied = JsonValue::parse(
            &file_dialog_host(vec![Capability::DialogOpenFile], SavingFileDialog).handle_json(
                &request_v1_8(
                    "dialog.save_file",
                    r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
                ),
            ),
        )
        .expect("denied save dialog response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_8(
            "dialog.save_file",
            r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
        )))
        .expect("invalid save dialog response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let unsupported = JsonValue::parse(&accepted_host.handle_json(&request_v1_7(
            "dialog.save_file",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("unsupported save dialog response is JSON");
        assert_eq!(
            field(field(&unsupported, "error"), "code").as_string(),
            Some("operation.unsupported")
        );
    }

    #[test]
    fn selection_dialog_requires_the_open_grant_and_returns_an_opaque_reference() {
        let accepted_host = file_access_host(
            vec![Capability::DialogOpenFile],
            CapturingFileDialog,
            FixedFileText(Err(FileTextServiceError::Unavailable)),
        );
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
            "dialog.open_file.v2",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("selection response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "selectionReference").as_string(),
            Some("AbCdEfGhIjKlMnOpQrStUv")
        );

        let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_9(
            "dialog.open_file.v2",
            r#"{"filters":[{"label":"Text","extensions":["txt"]}]}"#,
        )))
        .expect("denied response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
            "dialog.open_file.v2",
            r#"{"filters":[{"label":"Raw","extensions":["*.txt"]}]}"#,
        )))
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn selected_file_text_is_separately_granted_bounded_and_safe() {
        let reference = "AbCdEfGhIjKlMnOpQrStUv";
        let accepted_host = file_access_host(
            vec![Capability::FileReadText],
            CapturingFileDialog,
            FixedFileText(Ok("selected text".to_owned())),
        );
        let accepted = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
            "file.read_text",
            &format!(r#"{{"selectionReference":"{reference}"}}"#),
        )))
        .expect("text response is JSON");
        assert_eq!(field(&accepted, "status").as_string(), Some("success"));
        assert_eq!(
            field(field(&accepted, "result"), "text").as_string(),
            Some("selected text")
        );

        let denied = JsonValue::parse(&host(vec![Capability::DialogOpenFile]).handle_json(
            &request_v1_9(
                "file.read_text",
                &format!(r#"{{"selectionReference":"{reference}"}}"#),
            ),
        ))
        .expect("denied response is JSON");
        assert_eq!(
            field(field(&denied, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let invalid = JsonValue::parse(&accepted_host.handle_json(&request_v1_9(
            "file.read_text",
            r#"{"selectionReference":"path.txt"}"#,
        )))
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        for (service_error, expected) in [
            (FileTextServiceError::Unavailable, "file.unavailable"),
            (FileTextServiceError::InvalidText, "file.text_invalid"),
            (FileTextServiceError::TooLarge, "file.text_too_large"),
        ] {
            let failing_host = file_access_host(
                vec![Capability::FileReadText],
                CapturingFileDialog,
                FixedFileText(Err(service_error)),
            );
            let response = JsonValue::parse(&failing_host.handle_json(&request_v1_9(
                "file.read_text",
                &format!(r#"{{"selectionReference":"{reference}"}}"#),
            )))
            .expect("failure response is JSON");
            assert_eq!(
                field(field(&response, "error"), "code").as_string(),
                Some(expected)
            );
        }

        let oversized_host = file_access_host(
            vec![Capability::FileReadText],
            CapturingFileDialog,
            FixedFileText(Ok("x".repeat(MAX_FILE_TEXT_RESPONSE_BYTES + 1))),
        );
        let oversized = JsonValue::parse(&oversized_host.handle_json(&request_v1_9(
            "file.read_text",
            &format!(r#"{{"selectionReference":"{reference}"}}"#),
        )))
        .expect("oversized response is JSON");
        assert_eq!(
            field(field(&oversized, "error"), "code").as_string(),
            Some("file.text_too_large")
        );
    }

    #[test]
    fn storage_operations_are_exact_bounded_and_independently_granted() {
        let storage_host = storage_host(
            vec![
                Capability::StorageStateRead,
                Capability::StorageStateReplace,
                Capability::StorageStateClear,
            ],
            MemoryStorage::with_state(StorageRead::Absent),
        );
        let replaced = JsonValue::parse(&storage_host.handle_json(&request_v1_10(
            "storage.state.replace",
            r#"{"snapshot":"saved"}"#,
        )))
        .expect("replace response is JSON");
        assert_eq!(
            field(field(&replaced, "result"), "status").as_string(),
            Some("replaced")
        );

        let read =
            JsonValue::parse(&storage_host.handle_json(&request_v1_10("storage.state.read", "{}")))
                .expect("read response is JSON");
        assert_eq!(
            field(field(&read, "result"), "snapshot").as_string(),
            Some("saved")
        );

        let cleared = JsonValue::parse(
            &storage_host.handle_json(&request_v1_10("storage.state.clear", "{}")),
        )
        .expect("clear response is JSON");
        assert_eq!(
            field(field(&cleared, "result"), "status").as_string(),
            Some("cleared")
        );

        let invalid = JsonValue::parse(
            &storage_host.handle_json(&request_v1_10("storage.state.read", r#"{"extra":true}"#)),
        )
        .expect("invalid response is JSON");
        assert_eq!(
            field(field(&invalid, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );

        let no_grant =
            JsonValue::parse(&host(vec![]).handle_json(&request_v1_10("storage.state.read", "{}")))
                .expect("denied response is JSON");
        assert_eq!(
            field(field(&no_grant, "error"), "code").as_string(),
            Some("capability.denied")
        );

        let oversized = format!(
            r#"{{"snapshot":"{}"}}"#,
            "x".repeat(MAX_STORAGE_SNAPSHOT_REQUEST_BYTES + 1)
        );
        let rejected = JsonValue::parse(
            &storage_host.handle_json(&request_v1_10("storage.state.replace", &oversized)),
        )
        .expect("oversized response is JSON");
        assert_eq!(
            field(field(&rejected, "error"), "code").as_string(),
            Some("request.payload_invalid")
        );
    }

    #[test]
    fn rejects_duplicate_host_capability_grants() {
        assert!(
            HostPolicy::new(
                "test.application",
                vec![Capability::DiagnosticsRead, Capability::DiagnosticsRead],
                "test-host",
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_unsupported_versions_and_oversized_messages() {
        let unsupported = request("platform.ping", r#"{"sentAt":"now"}"#).replacen(
            "\"major\":1",
            "\"major\":2",
            1,
        );
        let response =
            JsonValue::parse(&host(vec![]).handle_json(&unsupported)).expect("valid JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("protocol.version_unsupported")
        );

        let response =
            JsonValue::parse(&host(vec![]).handle_json(&"x".repeat(MAX_REQUEST_BYTES + 1)))
                .expect("valid JSON");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.invalid")
        );
    }

    #[test]
    fn converts_known_epoch_days_without_a_time_library() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_300), (2025, 7, 31));
    }
}
