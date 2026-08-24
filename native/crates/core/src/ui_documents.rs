//! Session document and logical-view lifecycle handlers.
//!
//! This module owns the bounded document replacement and secondary-view
//! operations. It works only with session-local identifiers and the portable
//! window group; native window creation and destruction remain host concerns.

use super::*;

impl CoreHost {
    pub(super) fn handle_ui_document_replace(
        &self,
        request: RequestEnvelope,
        format: UiDocumentFormat,
    ) -> JsonValue {
        let operation = format.document_operation();
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

        let snapshot = if let Some(group) = &self.ui_window_group {
            let primary = UiWindowId::primary();
            let replacement = match format {
                UiDocumentFormat::V1 => group.replace_document(&primary, document),
                UiDocumentFormat::V2 => group.replace_document_v2(&primary, document),
                UiDocumentFormat::V3 => group.replace_document_v3(&primary, document),
            };
            replacement.ok().map(|snapshot| snapshot.snapshot().clone())
        } else {
            let session = self
                .ui_document_session
                .as_ref()
                .expect("legacy core has a primary document session");
            let mut session = session.borrow_mut();
            let revision = match format {
                UiDocumentFormat::V1 => session.replace_document(document),
                UiDocumentFormat::V2 => session.replace_document_v2(document),
                UiDocumentFormat::V3 => session.replace_document_v3(document),
            };
            revision.ok().and_then(|revision| {
                session
                    .snapshot()
                    .filter(|snapshot| snapshot.revision() == revision)
            })
        };
        let Some(snapshot) = snapshot else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document is invalid."),
                None,
            );
        };
        let revision = snapshot.revision();
        if let Some(update) = &self.pending_ui_document_update {
            *update.borrow_mut() = Some(snapshot);
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([("revision", JsonValue::String(revision.value().to_string()))]),
        )
    }

    /// Opens one bounded secondary view in this authenticated session group.
    ///
    /// The worker owns no native window. It can wait only for the portable
    /// group to report that the host UI thread created and registered one
    /// private native view. A successful opaque ID is therefore never a
    /// speculative reservation or a native handle.
    pub(super) fn handle_window_open(
        &self,
        request: RequestEnvelope,
        format: UiDocumentFormat,
    ) -> JsonValue {
        let operation = format.open_operation();
        let Some((title, document)) = window_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} requires one title and one document string."),
                None,
            );
        };
        if !self.policy.has(Capability::WindowOpen) {
            return self.capability_denied(request.request_id, operation);
        }
        if !self.policy.has(Capability::UiDocumentWrite) {
            return self.capability_denied(request.request_id, "ui.document.write");
        }
        if document.len() > MAX_UI_DOCUMENT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document exceeded the operation size limit."),
                None,
            );
        }
        let Ok(title) = WindowTitleProposal::new(title) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowTitleInvalid,
                format!("{operation} title is invalid."),
                None,
            );
        };
        let Some(group) = &self.ui_window_group else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            );
        };
        let opened = match format {
            UiDocumentFormat::V1 => group.open_secondary(title, document),
            UiDocumentFormat::V2 => group.open_secondary_v2(title, document),
            UiDocumentFormat::V3 => group.open_secondary_v3(title, document),
        };
        match opened {
            Ok(id) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("windowId", JsonValue::String(id.to_protocol_string()))]),
            ),
            Err(UiWindowGroupError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "another session window creation is pending.",
                None,
            ),
            Err(UiWindowGroupError::DocumentRejected(_)) => self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document is invalid."),
                None,
            ),
            Err(UiWindowGroupError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            ),
        }
    }

    /// Requests a host-owned close for one current secondary view.
    ///
    /// The protocol acknowledges queueing only. Windows may still be
    /// processing the request, and the actual logical view remains available
    /// until the native destroy path removes its private mapping.
    pub(super) fn handle_window_close(&self, request: RequestEnvelope) -> JsonValue {
        let Some(id) = secondary_window_id_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.close requires one canonical secondary windowId.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowClose) {
            return self.capability_denied(request.request_id, "window.close");
        }
        let Some(group) = &self.ui_window_group else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            );
        };
        if group.request_secondary_close(&id).is_err() {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the requested session window is unavailable.",
                None,
            );
        }
        ResponseEnvelope::success(
            request.request_id,
            &self.policy.host_name,
            object([("status", JsonValue::String("requested".to_owned()))]),
        )
    }

    /// Replaces the strict v1 document of one logical session view.
    ///
    /// `main` remains a legal target here so callers can keep one uniform
    /// document-update path after opening a secondary. Closing `main` remains
    /// forbidden: it is the group anchor rather than an ordinary target.
    pub(super) fn handle_ui_document_replace_window(
        &self,
        request: RequestEnvelope,
        format: UiDocumentFormat,
    ) -> JsonValue {
        let operation = format.window_operation();
        let Some((id, document)) = window_document_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} requires one canonical windowId and document string."),
                None,
            );
        };
        if !self.policy.has(Capability::UiDocumentWrite) {
            return self.capability_denied(request.request_id, "ui.document.write");
        }
        if document.len() > MAX_UI_DOCUMENT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document exceeded the operation size limit."),
                None,
            );
        }
        let Some(group) = &self.ui_window_group else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            );
        };
        let replacement = match format {
            UiDocumentFormat::V1 => group.replace_document(&id, document),
            UiDocumentFormat::V2 => group.replace_document_v2(&id, document),
            UiDocumentFormat::V3 => group.replace_document_v3(&id, document),
        };
        match replacement {
            Ok(snapshot) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([(
                    "revision",
                    JsonValue::String(snapshot.snapshot().revision().value().to_string()),
                )]),
            ),
            Err(UiWindowGroupError::DocumentRejected(_)) => self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                format!("{operation} document is invalid."),
                None,
            ),
            Err(UiWindowGroupError::Busy | UiWindowGroupError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the requested session window is unavailable.",
                None,
            ),
        }
    }
}

fn ui_document_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("document"))
        .flatten()
        .and_then(JsonValue::as_string)
}

/// Reads the exact two-field initial secondary-window payload.
///
/// Extra fields stay invalid so an application cannot smuggle a position,
/// size, parent, native style, handle, or any other desktop control into the
/// first deliberately small window-creation contract.
fn window_open_payload(value: &JsonValue) -> Option<(&str, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    Some((
        fields.get("title")?.as_string()?,
        fields.get("document")?.as_string()?,
    ))
}

/// Reads one exact secondary-window close payload.
///
/// `main` is intentionally rejected here before any host state is consulted:
/// it is the session anchor and ends only through the separately granted
/// `session.close` operation.
fn secondary_window_id_payload(value: &JsonValue) -> Option<UiWindowId> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    let id = UiWindowId::parse(fields.get("windowId")?.as_string()?).ok()?;
    (!id.is_primary()).then_some(id)
}

/// Reads one exact strict-v1 document update targeted at a logical view.
///
/// `main` is allowed, which lets applications use a uniform known-view update
/// method. The host still resolves it only inside the current authenticated
/// group and never exposes a lookup or enumeration operation.
fn window_document_payload(value: &JsonValue) -> Option<(UiWindowId, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    Some((
        UiWindowId::parse(fields.get("windowId")?.as_string()?).ok()?,
        fields.get("document")?.as_string()?,
    ))
}
