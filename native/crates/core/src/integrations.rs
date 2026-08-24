//! Non-UI platform integration handlers.
//!
//! These operations validate small protocol values, check one explicit grant,
//! and delegate to an injected host service. They never expose native details.

use super::*;

impl CoreHost {
    pub(super) fn handle_diagnostics_entries_read(&self, request: RequestEnvelope) -> JsonValue {
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

    pub(super) fn handle_clipboard_read(&self, request: RequestEnvelope) -> JsonValue {
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

    pub(super) fn handle_clipboard_write(&self, request: RequestEnvelope) -> JsonValue {
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

    pub(super) fn handle_external_open(&self, request: RequestEnvelope) -> JsonValue {
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

    /// Performs one host-authorized, bounded HTTPS text fetch.
    ///
    /// The core validates only the protocol URL and grant. The injected native
    /// service owns exact-origin policy and returns only public-safe result
    /// categories, so this layer cannot expose network diagnostics.
    pub(super) fn handle_network_fetch_text(&self, request: RequestEnvelope) -> JsonValue {
        let Some(url) = network_fetch_text_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "network.fetch_text requires one bounded URL string.",
                None,
            );
        };
        if url.len() > MAX_NETWORK_FETCH_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "network.fetch_text URL exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::NetworkFetch) {
            return self.capability_denied(request.request_id, "network.fetch");
        }
        let url = match NetworkUrl::parse(url) {
            Ok(url) => url,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "network.fetch_text URL is invalid.",
                    None,
                );
            }
        };
        match self.network.fetch_text(&url) {
            Ok(response) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    (
                        "statusCode",
                        JsonValue::Number(response.status_code().to_string()),
                    ),
                    ("text", JsonValue::String(response.text().to_owned())),
                ]),
            ),
            Err(NetworkTextServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::NetworkUnavailable,
                "network text fetch is unavailable.",
                None,
            ),
            Err(NetworkTextServiceError::ResponseInvalid) => self.failure(
                request.request_id,
                ProtocolErrorCode::NetworkResponseInvalid,
                "network text response is invalid.",
                None,
            ),
        }
    }

    /// Shows one bounded notification for an authenticated session.
    ///
    /// The result reports only that the host handed the values over. It must
    /// never describe what the user experienced.
    pub(super) fn handle_notification_show(&self, request: RequestEnvelope) -> JsonValue {
        let Some((title, body)) = notification_show_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "notification.show requires one title and one body string.",
                None,
            );
        };
        if !self.policy.has(Capability::NotificationShow) {
            return self.capability_denied(request.request_id, "notification.show");
        }

        // Validation failures never echo the offending text back: a rejected
        // notification must not become a way to have the host repeat content.
        let (Ok(title), Ok(body)) = (NotificationTitle::new(title), NotificationBody::new(body))
        else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::NotificationTextInvalid,
                "notification.show text is invalid.",
                None,
            );
        };

        match self.notifications.show(&Notification::new(title, body)) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("shown".to_owned()))]),
            ),
            Err(NotificationServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::NotificationUnavailable,
                "notifications are unavailable.",
                None,
            ),
            Err(NotificationServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::NotificationBusy,
                "a notification is already pending.",
                None,
            ),
        }
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
}

fn clipboard_write_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("text"))
        .flatten()
        .and_then(JsonValue::as_string)
}

/// Reads the exact two-field payload `notification.show` accepts.
///
/// Any extra field is a mismatch rather than something to ignore, so a future
/// urgency, icon, or action field cannot be smuggled past this version.
fn notification_show_payload(value: &JsonValue) -> Option<(&str, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let title = fields.get("title").and_then(JsonValue::as_string)?;
    let body = fields.get("body").and_then(JsonValue::as_string)?;
    Some((title, body))
}

/// Reads the exact one-field payload `external.open` accepts.
///
/// Extra fields are a mismatch rather than a future target, browser option,
/// native command, or other escape hatch. The URL still receives explicit
/// scheme and size validation before it reaches the host service.
fn external_open_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("url"))
        .flatten()
        .and_then(JsonValue::as_string)
}

/// Reads the exact one-field payload `network.fetch_text` accepts.
///
/// Extra fields are a mismatch rather than a future method, body, header,
/// cookie, credential, proxy, redirect, timeout, or native-handle escape
/// hatch. That absence keeps the service a bounded data seam.
fn network_fetch_text_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("url"))
        .flatten()
        .and_then(JsonValue::as_string)
}
