#![forbid(unsafe_code)]

//! Policy-bound handling for one native protocol message.
//!
//! Transports authenticate their sessions before calling this module. Incoming
//! capability context is intentionally ignored: only the host-created policy
//! can authorize a privileged operation.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use anodrel_protocol::{
    Capability, JsonValue, ProtocolErrorCode, ProtocolVersion, RequestEnvelope, ResponseEnvelope,
    is_empty_object, object, sent_at,
};
use anodrel_ui_session::{UiDocumentSession, UiDocumentSnapshot, UiInputMailbox};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_UI_DOCUMENT_REQUEST_BYTES: usize = 24 * 1024;

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
    pending_ui_document_update: RefCell<Option<UiDocumentSnapshot>>,
}

impl CoreHost {
    pub fn new(policy: HostPolicy) -> Self {
        Self::with_ui_input_mailbox(policy, UiInputMailbox::new())
    }

    /// Creates a host core that validates semantic input from one supplied
    /// per-session mailbox.
    pub fn with_ui_input_mailbox(policy: HostPolicy, ui_input_mailbox: UiInputMailbox) -> Self {
        Self {
            policy,
            ui_document_session: RefCell::new(UiDocumentSession::new()),
            ui_input_mailbox,
            pending_ui_document_update: RefCell::new(None),
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
            "ui.document.replace" if request.protocol_version.minor >= 1 => {
                self.handle_ui_document_replace(request)
            }
            "ui.events.read" if request.protocol_version.minor >= 2 => {
                self.handle_ui_events_read(request)
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

    fn handle_ui_document_replace(&self, request: RequestEnvelope) -> JsonValue {
        let Some(document) = ui_document_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "ui.document.replace requires one document string.",
                None,
            );
        };
        if !self.policy.has(Capability::UiDocumentWrite) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::CapabilityDenied,
                "ui.document.replace requires the ui.document.write capability.",
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
                "ui.document.replace document exceeded the operation size limit.",
                None,
            );
        }

        let Some(snapshot) = ({
            let mut session = self.ui_document_session.borrow_mut();
            let revision = session.replace_document(document);
            revision.ok().and_then(|revision| {
                session
                    .snapshot()
                    .filter(|snapshot| snapshot.revision() == revision)
            })
        }) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "ui.document.replace document is invalid.",
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
    use anodrel_ui::{ElementId, UiEvent};
    use anodrel_ui_session::UiInputCandidate;

    use super::*;

    fn host(grants: Vec<Capability>) -> CoreHost {
        CoreHost::new(
            HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
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

    fn ui_document_payload(document: &str) -> String {
        object([("document", JsonValue::String(document.to_owned()))]).to_json()
    }

    fn valid_ui_document(label: &str) -> String {
        format!(
            r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"root","kind":"action","label":"{label}","fontSize":16,"enabled":true,"tone":"accent"}}}}"#
        )
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
