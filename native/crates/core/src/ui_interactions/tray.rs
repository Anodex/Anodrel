//! Protocol and event handling for one host-owned semantic tray menu.

use super::*;

const TRAY_ACTION_EVENT_SCHEMA_VERSION: ProtocolVersion = ProtocolVersion {
    major: 1,
    minor: 33,
};

impl CoreHost {
    pub(crate) fn handle_tray_replace(&self, request: RequestEnvelope) -> JsonValue {
        let Some(model) =
            semantic_menu::replace_payload(&request.payload, MAX_TRAY_REPLACE_REQUEST_BYTES)
        else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "tray.replace requires one exact bounded complete tray-menu model.",
                None,
            );
        };
        if !self.policy.has(Capability::TrayWrite) {
            return self.capability_denied(request.request_id, "tray.write");
        }

        let revision = match self.tray_session.borrow().next_revision() {
            Ok(revision) => revision,
            Err(_) => return tray_unavailable(self, request.request_id),
        };
        if self.tray.replace(revision, model.clone()).is_err() {
            return tray_unavailable(self, request.request_id);
        }
        match self.tray_session.borrow_mut().replace(model) {
            Ok(committed_revision) => {
                debug_assert_eq!(committed_revision, revision);
                ResponseEnvelope::success(
                    request.request_id,
                    &self.policy.host_name,
                    object([(
                        "revision",
                        JsonValue::String(committed_revision.value().to_string()),
                    )]),
                )
            }
            Err(_) => tray_unavailable(self, request.request_id),
        }
    }
}

pub(super) fn action_event(event: anodrel_menu::TrayActionEvent) -> JsonValue {
    event_json(event, None)
}

/// Adds the authenticated primary-view identity without disclosing a native window.
pub(super) fn window_action_event(
    id: &UiWindowId,
    event: anodrel_menu::TrayActionEvent,
) -> JsonValue {
    event_json(event, Some(id))
}

fn event_json(event: anodrel_menu::TrayActionEvent, id: Option<&UiWindowId>) -> JsonValue {
    let mut fields = vec![
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("tray.action.invoked".to_owned()),
        ),
        ("source", JsonValue::String("native.tray".to_owned())),
        ("schemaVersion", TRAY_ACTION_EVENT_SCHEMA_VERSION.to_json()),
    ];
    if let Some(id) = id {
        fields.push(("windowId", JsonValue::String(id.to_protocol_string())));
    }
    fields.push((
        "payload",
        object([
            (
                "trayRevision",
                JsonValue::String(event.revision().value().to_string()),
            ),
            (
                "action",
                JsonValue::String(event.action().as_str().to_owned()),
            ),
        ]),
    ));
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn tray_unavailable(host: &CoreHost, request_id: String) -> JsonValue {
    host.failure(
        request_id,
        ProtocolErrorCode::TrayUnavailable,
        "the session tray is unavailable.",
        None,
    )
}
