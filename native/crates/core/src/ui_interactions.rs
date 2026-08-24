//! Menu, semantic-input, session-close, and field-read handlers.
//!
//! These handlers transform only validated, session-local interaction state
//! into protocol events and responses. They do not inspect native windows or
//! expose input timing, focus, or desktop state.

use super::*;

const MENU_ACTION_EVENT_SCHEMA_VERSION: ProtocolVersion = ProtocolVersion {
    major: 1,
    minor: 18,
};

impl CoreHost {
    pub(super) fn handle_menu_replace(&self, request: RequestEnvelope) -> JsonValue {
        let Some(model) =
            menu_replace_payload(&request.payload, request.protocol_version.minor >= 24)
        else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "menu.replace requires one exact bounded complete menu model.",
                None,
            );
        };
        if !self.policy.has(Capability::MenuWrite) {
            return self.capability_denied(request.request_id, "menu.write");
        }

        let revision = match self.menu_session.borrow().next_revision() {
            Ok(revision) => revision,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::MenuUnavailable,
                    "the session menu is unavailable.",
                    None,
                );
            }
        };
        if self.menu.replace(revision, model.clone()).is_err() {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::MenuUnavailable,
                "the session menu is unavailable.",
                None,
            );
        }
        match self.menu_session.borrow_mut().replace(model) {
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
            Err(_) => self.failure(
                request.request_id,
                ProtocolErrorCode::MenuUnavailable,
                "the session menu is unavailable.",
                None,
            ),
        }
    }

    pub(super) fn handle_ui_events_read(&self, request: RequestEnvelope) -> JsonValue {
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

        let batch = if let Some(group) = &self.ui_window_group {
            match group.drain_input_batch(&UiWindowId::primary()) {
                Ok(batch) => batch,
                Err(_) => {
                    return self.failure(
                        request.request_id,
                        ProtocolErrorCode::WindowUnavailable,
                        "the session UI view is unavailable.",
                        None,
                    );
                }
            }
        } else {
            self.ui_input_mailbox
                .as_ref()
                .expect("legacy core has a primary input mailbox")
                .drain()
        };
        let dropped = batch.dropped();
        let mut discarded = 0_u32;
        let mut events = Vec::new();
        for candidate in batch.into_candidates() {
            match candidate {
                SessionInteractionCandidate::Ui(candidate) => {
                    let (revision, event) = candidate.into_parts();
                    let accepted = if let Some(group) = &self.ui_window_group {
                        group
                            .accept_event(&UiWindowId::primary(), revision, event)
                            .ok()
                    } else {
                        self.ui_document_session
                            .as_ref()
                            .expect("legacy core has a primary document session")
                            .borrow()
                            .accept_event(revision, event)
                            .ok()
                    };
                    match accepted {
                        Some(event) => events.push(ui_action_event(event)),
                        None => discarded = discarded.saturating_add(1),
                    }
                }
                SessionInteractionCandidate::Menu(candidate) => {
                    let (revision, action) = candidate.into_parts();
                    match self.menu_session.borrow().accept_action(revision, action) {
                        Ok(event) => events.push(menu_action_event(event)),
                        Err(_) => discarded = discarded.saturating_add(1),
                    }
                }
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

    /// Drains bounded semantic input from every current view in this session.
    ///
    /// Batches retain their own view-local input order. The group deliberately
    /// makes no cross-view timing claim, even though its private iteration is
    /// deterministic for testability. Every accepted event receives an opaque
    /// `windowId` tag so application code can validate it against the view
    /// identity it created without learning any native state.
    pub(super) fn handle_ui_events_read_window(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "ui.events.read.window does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::UiEventsRead) {
            return self.capability_denied(request.request_id, "ui.events.read");
        }
        let Some(group) = &self.ui_window_group else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "the session window group is unavailable.",
                None,
            );
        };

        let mut dropped = 0_u32;
        let mut discarded = 0_u32;
        let mut events = Vec::new();
        for window_batch in group.drain_input_batches() {
            let (id, batch) = window_batch.into_parts();
            dropped = dropped.saturating_add(batch.dropped());
            for candidate in batch.into_candidates() {
                match candidate {
                    SessionInteractionCandidate::Ui(candidate) => {
                        let (revision, event) = candidate.into_parts();
                        match group.accept_event(&id, revision, event) {
                            Ok(event) => events.push(window_ui_action_event(&id, event)),
                            Err(_) => discarded = discarded.saturating_add(1),
                        }
                    }
                    SessionInteractionCandidate::Menu(candidate) if id.is_primary() => {
                        let (revision, action) = candidate.into_parts();
                        match self.menu_session.borrow().accept_action(revision, action) {
                            Ok(event) => events.push(window_menu_action_event(&id, event)),
                            Err(_) => discarded = discarded.saturating_add(1),
                        }
                    }
                    SessionInteractionCandidate::Menu(_) => {
                        // A secondary receives no menu bridge. If a malformed
                        // host route ever places one there, fail it closed.
                        discarded = discarded.saturating_add(1);
                    }
                }
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

    pub(super) fn handle_session_close(&self, request: RequestEnvelope) -> JsonValue {
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

    /// Reads every field value on this session's own current surface.
    ///
    /// The payload is exactly `{}`. There is no selector, and that is the
    /// security property rather than a simplification: a caller able to narrow
    /// a read to one field could repeat it until the typing was reconstructed.
    /// Returning the whole surface makes every read cost the same, so reading
    /// often gains nothing. See `docs/UI_FIELDS.md` and Decision 0067.
    pub(super) fn handle_ui_fields_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "ui.fields.read accepts no payload fields.",
                None,
            );
        }
        if !self.policy.has(Capability::UiFieldsRead) {
            return self.capability_denied(request.request_id, "ui.fields.read");
        }

        match self.ui_fields.read() {
            Ok(snapshot) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([(
                    "fields",
                    JsonValue::Array(
                        snapshot
                            .fields()
                            .iter()
                            .map(|field| {
                                object([
                                    ("id", JsonValue::String(field.id().as_str().to_owned())),
                                    ("value", JsonValue::String(field.value().to_owned())),
                                ])
                            })
                            .collect(),
                    ),
                )]),
            ),
            Err(UiFieldReadError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::UiFieldsUnavailable,
                "no field values are available.",
                None,
            ),
        }
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

/// Builds one v1.25 view-tagged UI action without exposing any native window
/// fact. The tag is an opaque session-local identity, not a handle or a lookup
/// key outside this authenticated group.
fn window_ui_action_event(
    id: &UiWindowId,
    event: anodrel_ui_session::UiApplicationEvent,
) -> JsonValue {
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
        ("windowId", JsonValue::String(id.to_protocol_string())),
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

fn menu_action_event(event: anodrel_menu::MenuActionEvent) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("menu.action.invoked".to_owned()),
        ),
        ("source", JsonValue::String("native.menu".to_owned())),
        ("schemaVersion", MENU_ACTION_EVENT_SCHEMA_VERSION.to_json()),
        (
            "payload",
            object([
                (
                    "menuRevision",
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

/// Builds one v1.25 primary-view-tagged menu action. Menu ownership is still
/// primary-only; the tag makes that fact explicit without reporting a native
/// menu, shortcut, focus state, or window handle.
fn window_menu_action_event(id: &UiWindowId, event: anodrel_menu::MenuActionEvent) -> JsonValue {
    object([
        ("protocolVersion", ProtocolVersion::CURRENT.to_json()),
        ("kind", JsonValue::String("event".to_owned())),
        (
            "eventName",
            JsonValue::String("menu.action.invoked".to_owned()),
        ),
        ("source", JsonValue::String("native.menu".to_owned())),
        ("schemaVersion", MENU_ACTION_EVENT_SCHEMA_VERSION.to_json()),
        ("windowId", JsonValue::String(id.to_protocol_string())),
        (
            "payload",
            object([
                (
                    "menuRevision",
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

fn menu_replace_payload(value: &JsonValue, shortcuts_allowed: bool) -> Option<MenuModel> {
    if value.to_json().len() > MAX_MENU_REPLACE_REQUEST_BYTES {
        return None;
    }
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    let JsonValue::Array(menus) = fields.get("menus")? else {
        return None;
    };
    let menus = menus
        .iter()
        .map(|menu| {
            let fields = menu.as_object()?;
            if fields.len() != 2 {
                return None;
            }
            let label = MenuText::new(fields.get("label")?.as_string()?.to_owned()).ok()?;
            let JsonValue::Array(items) = fields.get("items")? else {
                return None;
            };
            let items = items
                .iter()
                .map(|item| {
                    let fields = item.as_object()?;
                    let shortcut = fields.get("shortcut");
                    if fields.len() != 3 + usize::from(shortcut.is_some())
                        || (!shortcuts_allowed && shortcut.is_some())
                    {
                        return None;
                    }
                    let id = MenuActionId::new(fields.get("id")?.as_string()?.to_owned()).ok()?;
                    let label = MenuText::new(fields.get("label")?.as_string()?.to_owned()).ok()?;
                    let JsonValue::Bool(enabled) = fields.get("enabled")? else {
                        return None;
                    };
                    let action = MenuAction::new(id, label, *enabled);
                    match shortcut {
                        Some(shortcut) => Some(
                            action.with_shortcut(MenuShortcut::parse(shortcut.as_string()?).ok()?),
                        ),
                        None => Some(action),
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            Menu::new(label, items).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    MenuModel::new(menus).ok()
}
