//! Protocol and event handling for one host-owned semantic context menu.

use super::*;

const CONTEXT_MENU_ACTION_EVENT_SCHEMA_VERSION: ProtocolVersion = ProtocolVersion {
    major: 1,
    minor: 32,
};

impl CoreHost {
    pub(crate) fn handle_context_menu_replace(&self, request: RequestEnvelope) -> JsonValue {
        let Some(model) = semantic_menu::replace_payload(
            &request.payload,
            MAX_CONTEXT_MENU_REPLACE_REQUEST_BYTES,
        ) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "menu.context.replace requires one exact bounded complete context-menu model.",
                None,
            );
        };
        if !self.policy.has(Capability::ContextMenuWrite) {
            return self.capability_denied(request.request_id, "menu.context.write");
        }

        let revision = match self.context_menu_session.borrow().next_revision() {
            Ok(revision) => revision,
            Err(_) => return context_menu_unavailable(self, request.request_id),
        };
        if self.context_menu.replace(revision, model.clone()).is_err() {
            return context_menu_unavailable(self, request.request_id);
        }
        match self.context_menu_session.borrow_mut().replace(model) {
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
            Err(_) => context_menu_unavailable(self, request.request_id),
        }
    }
}

pub(super) fn action_event(event: anodrel_menu::ContextMenuActionEvent) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("menu.context.action.invoked".to_owned()),
        ),
        (
            "source",
            JsonValue::String("native.context_menu".to_owned()),
        ),
        (
            "schemaVersion",
            CONTEXT_MENU_ACTION_EVENT_SCHEMA_VERSION.to_json(),
        ),
        (
            "payload",
            object([
                (
                    "contextMenuRevision",
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

/// Adds the authenticated primary-view identity without disclosing a native window.
pub(super) fn window_action_event(
    id: &UiWindowId,
    event: anodrel_menu::ContextMenuActionEvent,
) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("menu.context.action.invoked".to_owned()),
        ),
        (
            "source",
            JsonValue::String("native.context_menu".to_owned()),
        ),
        (
            "schemaVersion",
            CONTEXT_MENU_ACTION_EVENT_SCHEMA_VERSION.to_json(),
        ),
        ("windowId", JsonValue::String(id.to_protocol_string())),
        (
            "payload",
            object([
                (
                    "contextMenuRevision",
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

fn context_menu_unavailable(host: &CoreHost, request_id: String) -> JsonValue {
    host.failure(
        request_id,
        ProtocolErrorCode::MenuUnavailable,
        "the session context menu is unavailable.",
        None,
    )
}
