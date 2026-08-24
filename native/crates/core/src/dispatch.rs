//! Version-gated request entry points and shared response helpers.
//!
//! This module is the only operation-name dispatcher. Operation handlers live
//! in focused sibling modules and receive only an already-validated request.

use super::*;

impl CoreHost {
    /// Takes the latest accepted document snapshot not yet observed by the
    /// transport that owns this core host.
    pub fn take_ui_document_update(&self) -> Option<UiDocumentSnapshot> {
        self.pending_ui_document_update
            .as_ref()
            .and_then(|update| update.borrow_mut().take())
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

    /// Produces the safe result for a request whose cancellation was observed
    /// by the authenticated transport before this core began processing it.
    ///
    /// The transport obtains `request_id` only from a validated request
    /// envelope. This method does not retain cancellation state or attempt to
    /// roll back work that has already entered an operation handler.
    pub fn cancelled_response(&self, request_id: String) -> String {
        self.failure(
            request_id,
            ProtocolErrorCode::RequestCancelled,
            "Request was cancelled before the host began processing it.",
            None,
        )
        .to_json()
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
            "credential.read" if request.protocol_version.minor >= 12 => {
                self.handle_credential_read(request)
            }
            "credential.write" if request.protocol_version.minor >= 12 => {
                self.handle_credential_write(request)
            }
            "credential.delete" if request.protocol_version.minor >= 12 => {
                self.handle_credential_delete(request)
            }
            "notification.show" if request.protocol_version.minor >= 13 => {
                self.handle_notification_show(request)
            }
            "window.title.set" if request.protocol_version.minor >= 14 => {
                self.handle_window_title_set(request)
            }
            "ui.fields.read" if request.protocol_version.minor >= 15 => {
                self.handle_ui_fields_read(request)
            }
            "window.state.set" if request.protocol_version.minor >= 16 => {
                self.handle_window_state_set(request)
            }
            "window.focus.request" if request.protocol_version.minor >= 20 => {
                self.handle_window_focus_request(request)
            }
            "window.fullscreen.set" if request.protocol_version.minor >= 21 => {
                self.handle_window_fullscreen_set(request)
            }
            "window.size.set" if request.protocol_version.minor >= 23 => {
                self.handle_window_size_set(request)
            }
            "window.open" if request.protocol_version.minor >= 25 => {
                self.handle_window_open(request, UiDocumentFormat::V1)
            }
            "window.open.v2" if request.protocol_version.minor >= 27 => {
                self.handle_window_open(request, UiDocumentFormat::V2)
            }
            "window.open.v3" if request.protocol_version.minor >= 26 => {
                self.handle_window_open(request, UiDocumentFormat::V3)
            }
            "window.close" if request.protocol_version.minor >= 25 => {
                self.handle_window_close(request)
            }
            "menu.replace" if request.protocol_version.minor >= 18 => {
                self.handle_menu_replace(request)
            }
            "ui.document.replace" if request.protocol_version.minor >= 1 => {
                self.handle_ui_document_replace(request, UiDocumentFormat::V1)
            }
            "ui.document.replace.v2" if request.protocol_version.minor >= 4 => {
                self.handle_ui_document_replace(request, UiDocumentFormat::V2)
            }
            "ui.document.replace.v3" if request.protocol_version.minor >= 26 => {
                self.handle_ui_document_replace(request, UiDocumentFormat::V3)
            }
            "ui.document.replace.window" if request.protocol_version.minor >= 25 => {
                self.handle_ui_document_replace_window(request, UiDocumentFormat::V1)
            }
            "ui.document.replace.window.v2" if request.protocol_version.minor >= 27 => {
                self.handle_ui_document_replace_window(request, UiDocumentFormat::V2)
            }
            "ui.document.replace.window.v3" if request.protocol_version.minor >= 26 => {
                self.handle_ui_document_replace_window(request, UiDocumentFormat::V3)
            }
            "ui.events.read" if request.protocol_version.minor >= 2 => {
                self.handle_ui_events_read(request)
            }
            "ui.events.read.window" if request.protocol_version.minor >= 25 => {
                self.handle_ui_events_read_window(request)
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
            "network.fetch_text" if request.protocol_version.minor >= 19 => {
                self.handle_network_fetch_text(request)
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
            "dialog.save_file.v2" if request.protocol_version.minor >= 17 => {
                self.handle_file_dialog_save_with_reference(request)
            }
            "file.write_text" if request.protocol_version.minor >= 17 => {
                self.handle_file_text_write(request)
            }
            "file.write_binary" if request.protocol_version.minor >= 22 => {
                self.handle_file_binary_write(request)
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

    pub(super) fn capability_denied(&self, request_id: String, capability: &str) -> JsonValue {
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

    pub(super) fn failure(
        &self,
        request_id: String,
        code: ProtocolErrorCode,
        message: impl Into<String>,
        details: Option<BTreeMap<String, JsonValue>>,
    ) -> JsonValue {
        ResponseEnvelope::failure(request_id, &self.policy.host_name, code, message, details)
    }
}
