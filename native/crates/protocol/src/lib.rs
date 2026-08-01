#![forbid(unsafe_code)]

//! The native representation of Anodrel's versioned JSON protocol.
//!
//! It deliberately accepts additive unknown fields while validating every field
//! the host relies on. Capability declarations in an incoming message are not
//! authority; the core receives policy from the host process instead.

use std::collections::BTreeMap;

pub use anodrel_json::JsonValue;

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 5;
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
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolErrorCode {
    CapabilityDenied,
    OperationUnsupported,
    ProtocolVersionUnsupported,
    RequestInvalid,
    RequestPayloadInvalid,
    ClipboardUnavailable,
    ClipboardTextInvalid,
    ClipboardTextTooLarge,
}

impl ProtocolErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityDenied => "capability.denied",
            Self::OperationUnsupported => "operation.unsupported",
            Self::ProtocolVersionUnsupported => "protocol.version_unsupported",
            Self::RequestInvalid => "request.invalid",
            Self::RequestPayloadInvalid => "request.payload_invalid",
            Self::ClipboardUnavailable => "clipboard.unavailable",
            Self::ClipboardTextInvalid => "clipboard.text_invalid",
            Self::ClipboardTextTooLarge => "clipboard.text_too_large",
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
}
