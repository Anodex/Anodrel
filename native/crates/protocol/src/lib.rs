#![forbid(unsafe_code)]

//! The native representation of Anodrel's versioned JSON protocol.
//!
//! It deliberately accepts additive unknown fields while validating every field
//! the host relies on. Capability declarations in an incoming message are not
//! authority; the core receives policy from the host process instead.

use std::collections::BTreeMap;

pub use anodrel_json::JsonValue;

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 24;
pub const MAX_REQUEST_ID_BYTES: usize = 256;
pub const MAX_OPERATION_BYTES: usize = 128;
pub const MAX_CANCELLATION_ID_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };

    pub const fn is_supported(self) -> bool {
        self.major == PROTOCOL_MAJOR && self.minor <= PROTOCOL_MINOR
    }

    pub fn to_json(self) -> JsonValue {
        object([
            ("major", JsonValue::Number(self.major.to_string())),
            ("minor", JsonValue::Number(self.minor.to_string())),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    DiagnosticsRead,
    UiDocumentWrite,
    UiEventsRead,
    SessionClose,
    ClipboardRead,
    ClipboardWrite,
    ExternalOpen,
    /// Fetch bounded text from one host-authorized HTTPS origin.
    ///
    /// The separate host service owns exact origin policy, direct network
    /// calls, and all native state. This capability accepts no method, body,
    /// header, cookie, credential, proxy, or connection handle.
    NetworkFetch,
    DialogOpenFile,
    DialogSaveFile,
    FileReadText,
    FileWriteText,
    /// Write bounded decoded binary data through one retained output object.
    ///
    /// The grant has no path, file handle, stream, offset, or raw protocol
    /// byte surface; see `docs/FILE_BINARY_WRITE.md` and Decision 0087.
    FileWriteBinary,
    StorageStateRead,
    StorageStateReplace,
    StorageStateClear,
    CredentialRead,
    CredentialWrite,
    CredentialDelete,
    /// Show one bounded notification. There is no read counterpart to grant,
    /// because a notification has no read surface at all.
    NotificationShow,
    /// Propose the title of the session's own window.
    ///
    /// A proposal, not an assignment: the host composes the displayed caption
    /// with a validated application-name suffix. There is no read counterpart
    /// and no way to name a window. See `docs/WINDOW_TITLE.md`.
    WindowTitle,
    /// Request one standard presentation state for the session's own window.
    ///
    /// The state is a closed minimise/maximise/restore value. There is no
    /// target, readback, event, or native command surface. See
    /// `docs/WINDOW_STATE.md` and Decision 0072.
    WindowState,
    /// Ask Windows to foreground the session's own host window.
    ///
    /// The request has no target or focus-state readback. Windows may refuse
    /// it under its foreground rules; see `docs/WINDOW_FOCUS.md`.
    WindowFocus,
    /// Choose reversible borderless fullscreen for the session's own window.
    ///
    /// The host retains every native style and placement fact. This grant
    /// cannot select a monitor, change a display mode, set geometry, or read
    /// window state; see `docs/WINDOW_FULLSCREEN.md`.
    WindowFullscreen,
    /// Resize the client area of the session's own native window.
    ///
    /// The grant carries only bounded logical client dimensions. It cannot
    /// target or move a window, select a monitor, read geometry, or expose a
    /// native rectangle; see `docs/WINDOW_SIZE.md` and Decision 0088.
    WindowSize,
    /// Read every field value on the session's own current surface.
    ///
    /// A snapshot, not a stream. There is no selector and no change event, so
    /// this grant cannot be used to reconstruct what someone is typing. See
    /// `docs/UI_FIELDS.md` and Decision 0067.
    UiFieldsRead,
    /// Replace the complete native menu for this authenticated session.
    ///
    /// The application supplies semantic labels and enabled command IDs only.
    /// A host owns native identifiers, window attachment, and activation
    /// routing; see `docs/MENUS.md`.
    MenuWrite,
}

impl Capability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DiagnosticsRead => "diagnostics.read",
            Self::UiDocumentWrite => "ui.document.write",
            Self::UiEventsRead => "ui.events.read",
            Self::SessionClose => "session.close",
            Self::ClipboardRead => "clipboard.read",
            Self::ClipboardWrite => "clipboard.write",
            Self::ExternalOpen => "external.open",
            Self::NetworkFetch => "network.fetch",
            Self::DialogOpenFile => "dialog.open_file",
            Self::DialogSaveFile => "dialog.save_file",
            Self::FileReadText => "file.read_text",
            Self::FileWriteText => "file.write_text",
            Self::FileWriteBinary => "file.write_binary",
            Self::StorageStateRead => "storage.state.read",
            Self::StorageStateReplace => "storage.state.replace",
            Self::StorageStateClear => "storage.state.clear",
            Self::CredentialRead => "credential.read",
            Self::CredentialWrite => "credential.write",
            Self::CredentialDelete => "credential.delete",
            Self::NotificationShow => "notification.show",
            Self::WindowTitle => "window.title",
            Self::WindowState => "window.state",
            Self::WindowFocus => "window.focus",
            Self::WindowFullscreen => "window.fullscreen",
            Self::WindowSize => "window.size",
            Self::UiFieldsRead => "ui.fields.read",
            Self::MenuWrite => "menu.write",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolErrorCode {
    CapabilityDenied,
    OperationUnsupported,
    ProtocolVersionUnsupported,
    RequestCancelled,
    RequestInvalid,
    RequestPayloadInvalid,
    ClipboardUnavailable,
    ClipboardTextInvalid,
    ClipboardTextTooLarge,
    ExternalUnavailable,
    /// The host has no authorized text-fetch service, the origin is not
    /// allowed, or the direct native request could not complete.
    NetworkUnavailable,
    /// A native response could not be represented as the bounded public
    /// status-and-UTF-8-text value.
    NetworkResponseInvalid,
    DialogUnavailable,
    FileUnavailable,
    FileTextInvalid,
    FileTextTooLarge,
    /// Canonical decoded binary output exceeded its fixed request bound.
    FileBinaryTooLarge,
    StorageUnavailable,
    StorageSnapshotInvalid,
    StorageSnapshotTooLarge,
    DiagnosticsUnavailable,
    CredentialUnavailable,
    CredentialAccessDenied,
    CredentialStoredSecretInvalid,
    /// The host cannot show notifications, or the system refused. This never
    /// distinguishes a muted application from a busy shell.
    NotificationUnavailable,
    /// Another notification for this session is still pending.
    NotificationBusy,
    /// The supplied title or body failed the documented bounds or character
    /// rules. The failure never echoes the offending text back.
    NotificationTextInvalid,
    /// This session has no host window to title, or the native call failed.
    ///
    /// One code for both, deliberately: which it is describes host state an
    /// application has no business learning.
    WindowUnavailable,
    /// Another window-title proposal for this session is still pending.
    WindowBusy,
    /// The proposed title failed the documented bounds or character rules. The
    /// failure never echoes the offending text back.
    WindowTitleInvalid,
    /// This session has no surface whose field values can be read.
    ///
    /// One code for every reason. Distinguishing "no surface" from "no fields"
    /// from "the host was busy" would report state that, read repeatedly,
    /// describes what the person is doing.
    UiFieldsUnavailable,
    /// The host has no session-owned native menu, or could not update it.
    ///
    /// This intentionally does not distinguish absent UI state, a busy UI
    /// thread, or an operating-system failure.
    MenuUnavailable,
}

impl ProtocolErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityDenied => "capability.denied",
            Self::OperationUnsupported => "operation.unsupported",
            Self::ProtocolVersionUnsupported => "protocol.version_unsupported",
            Self::RequestCancelled => "request.cancelled",
            Self::RequestInvalid => "request.invalid",
            Self::RequestPayloadInvalid => "request.payload_invalid",
            Self::ClipboardUnavailable => "clipboard.unavailable",
            Self::ClipboardTextInvalid => "clipboard.text_invalid",
            Self::ClipboardTextTooLarge => "clipboard.text_too_large",
            Self::ExternalUnavailable => "external.unavailable",
            Self::NetworkUnavailable => "network.unavailable",
            Self::NetworkResponseInvalid => "network.response_invalid",
            Self::DialogUnavailable => "dialog.unavailable",
            Self::FileUnavailable => "file.unavailable",
            Self::FileTextInvalid => "file.text_invalid",
            Self::FileTextTooLarge => "file.text_too_large",
            Self::FileBinaryTooLarge => "file.binary_too_large",
            Self::StorageUnavailable => "storage.unavailable",
            Self::StorageSnapshotInvalid => "storage.snapshot_invalid",
            Self::StorageSnapshotTooLarge => "storage.snapshot_too_large",
            Self::DiagnosticsUnavailable => "diagnostics.unavailable",
            Self::CredentialUnavailable => "credential.unavailable",
            Self::CredentialAccessDenied => "credential.access_denied",
            Self::CredentialStoredSecretInvalid => "credential.stored_secret_invalid",
            Self::NotificationUnavailable => "notification.unavailable",
            Self::NotificationBusy => "notification.busy",
            Self::NotificationTextInvalid => "notification.text_invalid",
            Self::WindowUnavailable => "window.unavailable",
            Self::WindowBusy => "window.busy",
            Self::WindowTitleInvalid => "window.title_invalid",
            Self::UiFieldsUnavailable => "ui.fields.unavailable",
            Self::MenuUnavailable => "menu.unavailable",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestEnvelope {
    pub protocol_version: ProtocolVersion,
    pub request_id: String,
    pub operation: String,
    pub payload: JsonValue,
    pub cancellation_id: Option<String>,
}

impl RequestEnvelope {
    pub fn from_json(value: JsonValue) -> Result<Self, RequestError> {
        let fields = value.as_object().ok_or(RequestError::Malformed)?;
        let protocol_version = protocol_version(fields)?;
        if required_string(fields, "kind")? != "request" {
            return Err(RequestError::Malformed);
        }
        let request_id = required_string(fields, "requestId")?.to_owned();
        let operation = required_string(fields, "operation")?.to_owned();
        let payload = fields
            .get("payload")
            .cloned()
            .ok_or(RequestError::Malformed)?;
        let cancellation_id = match fields.get("cancellationId") {
            Some(value) => Some(value.as_string().ok_or(RequestError::Malformed)?.to_owned()),
            None => None,
        };

        if !is_limited_identifier(&request_id, MAX_REQUEST_ID_BYTES)
            || !is_limited_identifier(&operation, MAX_OPERATION_BYTES)
            || cancellation_id
                .as_deref()
                .is_some_and(|value| !is_limited_identifier(value, MAX_CANCELLATION_ID_BYTES))
        {
            return Err(RequestError::Malformed);
        }

        Ok(Self {
            protocol_version,
            request_id,
            operation,
            payload,
            cancellation_id,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestError {
    Malformed,
}

/// A transport control that prevents one not-yet-started request from running.
///
/// Cancellation has no request identifier of its own and produces no response.
/// A transport may remember it only for a small, documented pending set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancellationEnvelope {
    pub protocol_version: ProtocolVersion,
    pub cancellation_id: String,
}

impl CancellationEnvelope {
    pub fn from_json(value: JsonValue) -> Result<Self, RequestError> {
        let fields = value.as_object().ok_or(RequestError::Malformed)?;
        let protocol_version = protocol_version(fields)?;
        if required_string(fields, "kind")? != "cancel" {
            return Err(RequestError::Malformed);
        }
        let cancellation_id = required_string(fields, "cancellationId")?.to_owned();
        if !is_limited_identifier(&cancellation_id, MAX_CANCELLATION_ID_BYTES) {
            return Err(RequestError::Malformed);
        }
        Ok(Self {
            protocol_version,
            cancellation_id,
        })
    }
}

pub struct ResponseEnvelope;

impl ResponseEnvelope {
    pub fn success(request_id: String, host_name: &str, result: JsonValue) -> JsonValue {
        object([
            ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
            ("kind", JsonValue::String("response".to_owned())),
            ("requestId", JsonValue::String(request_id)),
            ("status", JsonValue::String("success".to_owned())),
            ("result", result),
            (
                "diagnostics",
                object([("hostName", JsonValue::String(host_name.to_owned()))]),
            ),
        ])
    }

    pub fn failure(
        request_id: String,
        host_name: &str,
        code: ProtocolErrorCode,
        message: impl Into<String>,
        details: Option<BTreeMap<String, JsonValue>>,
    ) -> JsonValue {
        let mut error = BTreeMap::from([
            (
                "code".to_owned(),
                JsonValue::String(code.as_str().to_owned()),
            ),
            ("message".to_owned(), JsonValue::String(message.into())),
            ("retryable".to_owned(), JsonValue::Bool(false)),
        ]);
        if let Some(details) = details {
            error.insert("details".to_owned(), JsonValue::Object(details));
        }
        object([
            ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
            ("kind", JsonValue::String("response".to_owned())),
            ("requestId", JsonValue::String(request_id)),
            ("status", JsonValue::String("failure".to_owned())),
            ("error", JsonValue::Object(error)),
            (
                "diagnostics",
                object([("hostName", JsonValue::String(host_name.to_owned()))]),
            ),
        ])
    }
}

pub fn is_empty_object(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Object(fields) if fields.is_empty())
}

pub fn sent_at(value: &JsonValue) -> Option<&str> {
    value.as_object()?.get("sentAt")?.as_string()
}

pub fn object<const N: usize>(fields: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn protocol_version(fields: &BTreeMap<String, JsonValue>) -> Result<ProtocolVersion, RequestError> {
    let version = fields
        .get("protocolVersion")
        .and_then(JsonValue::as_object)
        .ok_or(RequestError::Malformed)?;
    Ok(ProtocolVersion {
        major: version
            .get("major")
            .and_then(JsonValue::as_u16)
            .ok_or(RequestError::Malformed)?,
        minor: version
            .get("minor")
            .and_then(JsonValue::as_u16)
            .ok_or(RequestError::Malformed)?,
    })
}

fn required_string<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<&'a str, RequestError> {
    fields
        .get(field)
        .and_then(JsonValue::as_string)
        .ok_or(RequestError::Malformed)
}

fn is_limited_identifier(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= maximum_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_additive_fields_but_not_malformed_fields() {
        let value = JsonValue::parse(
            r#"{"protocolVersion":{"major":1,"minor":0},"kind":"request","requestId":"one","operation":"platform.health","payload":{},"futureField":true}"#,
        )
        .expect("valid JSON");
        assert_eq!(
            RequestEnvelope::from_json(value)
                .expect("valid envelope")
                .request_id,
            "one"
        );

        let malformed = JsonValue::parse(
            r#"{"protocolVersion":{"major":1,"minor":0},"kind":"request","requestId":"","operation":"platform.health","payload":{}}"#,
        )
        .expect("valid JSON");
        assert_eq!(
            RequestEnvelope::from_json(malformed),
            Err(RequestError::Malformed)
        );
    }

    #[test]
    fn rejects_identifiers_that_could_expand_a_response_beyond_transport_limits() {
        let oversized_id = "x".repeat(MAX_REQUEST_ID_BYTES + 1);
        let value = JsonValue::parse(&format!(
            r#"{{"protocolVersion":{{"major":1,"minor":0}},"kind":"request","requestId":"{oversized_id}","operation":"platform.health","payload":{{}}}}"#
        ))
        .expect("JSON is valid");
        assert_eq!(
            RequestEnvelope::from_json(value),
            Err(RequestError::Malformed)
        );
    }

    #[test]
    fn parses_a_bounded_cancellation_control() {
        let value = JsonValue::parse(
            r#"{"protocolVersion":{"major":1,"minor":0},"kind":"cancel","cancellationId":"stop-before-start"}"#,
        )
        .expect("valid JSON");
        assert_eq!(
            CancellationEnvelope::from_json(value)
                .expect("control is valid")
                .cancellation_id,
            "stop-before-start"
        );

        let malformed = JsonValue::parse(
            r#"{"protocolVersion":{"major":1,"minor":0},"kind":"cancel","cancellationId":""}"#,
        )
        .expect("valid JSON");
        assert_eq!(
            CancellationEnvelope::from_json(malformed),
            Err(RequestError::Malformed)
        );
    }
}
