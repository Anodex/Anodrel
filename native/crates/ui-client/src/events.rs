//! Typed parsing for bounded document and menu semantic events.

use std::collections::BTreeMap;

use anodrel_json::JsonValue;
use anodrel_menu::MenuActionId;
use anodrel_ui::ElementId;
use anodrel_ui_session::{MAX_SESSION_WINDOWS, UI_INPUT_QUEUE_CAPACITY};

use crate::{ContextMenuRevision, DocumentRevision, MenuRevision, SessionWindowId, UiClientError};

/// The host's maximum number of interaction candidates from one primary view
/// per pull.
pub const MAX_ACTIONS_PER_BATCH: usize = UI_INPUT_QUEUE_CAPACITY;

/// The maximum tagged document actions a multi-window group can return per
/// pull.
///
/// Each of the bounded session group's views owns one
/// [`MAX_ACTIONS_PER_BATCH`]-candidate input queue. The host drains every
/// current view for `ui.events.read.window`, so its closed response bound is
/// the product of both portable limits.
pub const MAX_WINDOW_ACTIONS_PER_BATCH: usize = MAX_SESSION_WINDOWS * MAX_ACTIONS_PER_BATCH;

/// One current document action accepted by the host session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiAction {
    revision: DocumentRevision,
    action: String,
}

impl UiAction {
    /// Returns the exact document revision that produced this action.
    #[must_use]
    pub const fn revision(&self) -> DocumentRevision {
        self.revision
    }

    /// Returns the validated semantic action element ID.
    #[must_use]
    pub fn action(&self) -> &str {
        &self.action
    }
}

/// One document action tagged with the logical session view that produced it.
///
/// The tag is an opaque session-local identity, not a native lookup key. Event
/// order remains meaningful only within that one view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowUiAction {
    window: SessionWindowId,
    action: UiAction,
}

impl WindowUiAction {
    /// Returns the host-issued logical source view.
    #[must_use]
    pub const fn window(&self) -> SessionWindowId {
        self.window
    }

    /// Returns the revision-bound semantic action from that view.
    #[must_use]
    pub fn action(&self) -> &UiAction {
        &self.action
    }
}

/// One current native-menu action accepted by the host session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiMenuAction {
    revision: MenuRevision,
    action: MenuActionId,
}

/// One current host-owned context-menu action accepted by the session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiContextMenuAction {
    revision: ContextMenuRevision,
    action: MenuActionId,
}

impl UiContextMenuAction {
    /// Returns the exact complete context-menu revision that produced this action.
    #[must_use]
    pub const fn revision(&self) -> ContextMenuRevision {
        self.revision
    }

    /// Returns the validated semantic context-menu action ID.
    #[must_use]
    pub fn action(&self) -> &str {
        self.action.as_str()
    }
}

impl UiMenuAction {
    /// Returns the exact complete-menu revision that produced this action.
    #[must_use]
    pub const fn revision(&self) -> MenuRevision {
        self.revision
    }

    /// Returns the validated semantic menu action ID.
    #[must_use]
    pub fn action(&self) -> &str {
        self.action.as_str()
    }
}

/// One host-validated semantic event from the shared session interaction queue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiEvent {
    /// A document action from the host-controlled UI surface.
    DocumentAction(UiAction),
    /// A menu action from the host-controlled native menu bar.
    MenuAction(UiMenuAction),
    /// A local popup action from the host-controlled context-menu surface.
    ContextMenuAction(UiContextMenuAction),
}

/// One bounded `ui.events.read` result accepting every documented event shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiEventBatch {
    events: Vec<UiEvent>,
    dropped: u32,
    discarded: u32,
}

impl UiEventBatch {
    /// Returns current accepted events in host delivery order.
    #[must_use]
    pub fn events(&self) -> &[UiEvent] {
        &self.events
    }

    /// Returns candidates dropped because the host interaction queue was full.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Returns candidates discarded by host revision/action validation.
    #[must_use]
    pub const fn discarded(&self) -> u32 {
        self.discarded
    }

    pub(crate) fn parse(result: &JsonValue) -> Result<Self, UiClientError> {
        let fields = result.as_object().ok_or(UiClientError::ResponseInvalid)?;
        let events = fields
            .get("events")
            .and_then(as_array)
            .ok_or(UiClientError::ResponseInvalid)?;
        if events.len() > MAX_ACTIONS_PER_BATCH {
            return Err(UiClientError::ResponseInvalid);
        }
        let dropped = fields
            .get("dropped")
            .and_then(as_canonical_u32)
            .ok_or(UiClientError::ResponseInvalid)?;
        let discarded = fields
            .get("discarded")
            .and_then(as_canonical_u32)
            .ok_or(UiClientError::ResponseInvalid)?;
        let events = events
            .iter()
            .map(UiEvent::parse)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            events,
            dropped,
            discarded,
        })
    }
}

/// One bounded document-only `ui.events.read` result.
///
/// This preserves the regular three-grant template API. A native-menu event is
/// not silently discarded or coerced into a document action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiActionBatch {
    actions: Vec<UiAction>,
    dropped: u32,
    discarded: u32,
}

impl UiActionBatch {
    /// Returns current document actions in host delivery order.
    #[must_use]
    pub fn actions(&self) -> &[UiAction] {
        &self.actions
    }

    /// Returns candidates dropped because the host interaction queue was full.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Returns candidates discarded by host revision/action validation.
    #[must_use]
    pub const fn discarded(&self) -> u32 {
        self.discarded
    }

    pub(crate) fn parse(result: &JsonValue) -> Result<Self, UiClientError> {
        let batch = UiEventBatch::parse(result)?;
        let actions = batch
            .events
            .into_iter()
            .map(|event| match event {
                UiEvent::DocumentAction(action) => Ok(action),
                UiEvent::MenuAction(_) => Err(UiClientError::ResponseInvalid),
                UiEvent::ContextMenuAction(_) => Err(UiClientError::ResponseInvalid),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            actions,
            dropped: batch.dropped,
            discarded: batch.discarded,
        })
    }
}

/// One bounded context-menu-only `ui.events.read` result.
///
/// This preserves the existing document and menu APIs: neither silently
/// accepts a new local-popup event kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiContextMenuActionBatch {
    actions: Vec<UiContextMenuAction>,
    dropped: u32,
    discarded: u32,
}

impl UiContextMenuActionBatch {
    /// Returns current context-menu actions in host delivery order.
    #[must_use]
    pub fn actions(&self) -> &[UiContextMenuAction] {
        &self.actions
    }

    /// Returns candidates dropped because the host interaction queue was full.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Returns candidates discarded by host revision/action validation.
    #[must_use]
    pub const fn discarded(&self) -> u32 {
        self.discarded
    }

    pub(crate) fn parse(result: &JsonValue) -> Result<Self, UiClientError> {
        let batch = UiEventBatch::parse(result)?;
        let actions = batch
            .events
            .into_iter()
            .map(|event| match event {
                UiEvent::ContextMenuAction(action) => Ok(action),
                UiEvent::DocumentAction(_) | UiEvent::MenuAction(_) => {
                    Err(UiClientError::ResponseInvalid)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            actions,
            dropped: batch.dropped,
            discarded: batch.discarded,
        })
    }
}

/// One bounded v1.25 action batch from every current session view.
///
/// This deliberately accepts only document actions. A menu action, malformed
/// window tag, unknown event, or noncanonical count becomes a closed client
/// error instead of being dropped or reinterpreted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowUiActionBatch {
    actions: Vec<WindowUiAction>,
    dropped: u32,
    discarded: u32,
}

impl WindowUiActionBatch {
    /// Returns typed actions in the host's bounded delivery order.
    #[must_use]
    pub fn actions(&self) -> &[WindowUiAction] {
        &self.actions
    }

    /// Returns the total candidates dropped by full view-local input queues.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Returns the total candidates discarded by host revalidation.
    #[must_use]
    pub const fn discarded(&self) -> u32 {
        self.discarded
    }

    pub(crate) fn parse(result: &JsonValue) -> Result<Self, UiClientError> {
        let fields = result.as_object().ok_or(UiClientError::ResponseInvalid)?;
        let events = fields
            .get("events")
            .and_then(as_array)
            .ok_or(UiClientError::ResponseInvalid)?;
        if events.len() > MAX_WINDOW_ACTIONS_PER_BATCH {
            return Err(UiClientError::ResponseInvalid);
        }
        let dropped = fields
            .get("dropped")
            .and_then(as_canonical_u32)
            .ok_or(UiClientError::ResponseInvalid)?;
        let discarded = fields
            .get("discarded")
            .and_then(as_canonical_u32)
            .ok_or(UiClientError::ResponseInvalid)?;
        let mut actions_per_window = BTreeMap::<SessionWindowId, usize>::new();
        let actions = events
            .iter()
            .map(|event| {
                let action = WindowUiAction::parse(event)?;
                let count = actions_per_window.entry(action.window()).or_default();
                *count += 1;
                if *count > MAX_ACTIONS_PER_BATCH {
                    return Err(UiClientError::ResponseInvalid);
                }
                Ok(action)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            actions,
            dropped,
            discarded,
        })
    }
}

impl UiEvent {
    fn parse(event: &JsonValue) -> Result<Self, UiClientError> {
        let fields = event.as_object().ok_or(UiClientError::ResponseInvalid)?;
        match (
            fields.get("eventName").and_then(JsonValue::as_string),
            fields.get("source").and_then(JsonValue::as_string),
        ) {
            (Some("ui.action.invoked"), Some("native.ui")) => {
                UiAction::parse(event).map(Self::DocumentAction)
            }
            (Some("menu.action.invoked"), Some("native.menu")) => {
                UiMenuAction::parse(event).map(Self::MenuAction)
            }
            (Some("menu.context.action.invoked"), Some("native.context_menu")) => {
                UiContextMenuAction::parse(event).map(Self::ContextMenuAction)
            }
            _ => Err(UiClientError::ResponseInvalid),
        }
    }
}

impl UiAction {
    fn parse(event: &JsonValue) -> Result<Self, UiClientError> {
        parse_ui_action(event, 2)
    }
}

impl WindowUiAction {
    fn parse(event: &JsonValue) -> Result<Self, UiClientError> {
        let fields = event.as_object().ok_or(UiClientError::ResponseInvalid)?;
        let window = fields
            .get("windowId")
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)
            .and_then(SessionWindowId::parse_event)?;
        let action = parse_ui_action(event, 25)?;
        Ok(Self { window, action })
    }
}

fn parse_ui_action(event: &JsonValue, minimum_minor: u16) -> Result<UiAction, UiClientError> {
    let fields = event.as_object().ok_or(UiClientError::ResponseInvalid)?;
    if fields.get("kind").and_then(JsonValue::as_string) != Some("event")
        || fields.get("eventName").and_then(JsonValue::as_string) != Some("ui.action.invoked")
        || fields.get("source").and_then(JsonValue::as_string) != Some("native.ui")
        || !is_version_at_least(fields.get("protocolVersion"), minimum_minor)
        || !is_exact_schema_v1(fields.get("schemaVersion"), 0)
    {
        return Err(UiClientError::ResponseInvalid);
    }
    let payload = fields
        .get("payload")
        .and_then(JsonValue::as_object)
        .ok_or(UiClientError::ResponseInvalid)?;
    let revision = payload
        .get("revision")
        .and_then(JsonValue::as_string)
        .ok_or(UiClientError::ResponseInvalid)
        .and_then(DocumentRevision::parse)?;
    let action = payload
        .get("action")
        .and_then(JsonValue::as_string)
        .ok_or(UiClientError::ResponseInvalid)?;
    ElementId::new(action.to_owned()).map_err(|_| UiClientError::ResponseInvalid)?;
    Ok(UiAction {
        revision,
        action: action.to_owned(),
    })
}

impl UiMenuAction {
    fn parse(event: &JsonValue) -> Result<Self, UiClientError> {
        let fields = event.as_object().ok_or(UiClientError::ResponseInvalid)?;
        if fields.get("kind").and_then(JsonValue::as_string) != Some("event")
            || !is_version_at_least(fields.get("protocolVersion"), 18)
            || !is_exact_schema_v1(fields.get("schemaVersion"), 18)
        {
            return Err(UiClientError::ResponseInvalid);
        }
        let payload = fields
            .get("payload")
            .and_then(JsonValue::as_object)
            .ok_or(UiClientError::ResponseInvalid)?;
        let revision = payload
            .get("menuRevision")
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)
            .and_then(MenuRevision::parse)?;
        let action = payload
            .get("action")
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)?;
        let action =
            MenuActionId::new(action.to_owned()).map_err(|_| UiClientError::ResponseInvalid)?;
        Ok(Self { revision, action })
    }
}

impl UiContextMenuAction {
    fn parse(event: &JsonValue) -> Result<Self, UiClientError> {
        let fields = event.as_object().ok_or(UiClientError::ResponseInvalid)?;
        if fields.get("kind").and_then(JsonValue::as_string) != Some("event")
            || !is_version_at_least(fields.get("protocolVersion"), 32)
            || !is_exact_schema_v1(fields.get("schemaVersion"), 32)
        {
            return Err(UiClientError::ResponseInvalid);
        }
        let payload = fields
            .get("payload")
            .and_then(JsonValue::as_object)
            .ok_or(UiClientError::ResponseInvalid)?;
        let revision = payload
            .get("contextMenuRevision")
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)
            .and_then(ContextMenuRevision::parse)?;
        let action = payload
            .get("action")
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)?;
        let action =
            MenuActionId::new(action.to_owned()).map_err(|_| UiClientError::ResponseInvalid)?;
        Ok(Self { revision, action })
    }
}

fn as_array(value: &JsonValue) -> Option<&[JsonValue]> {
    match value {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }
}

fn as_canonical_u32(value: &JsonValue) -> Option<u32> {
    let JsonValue::Number(value) = value else {
        return None;
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value
        .parse::<u32>()
        .ok()
        .filter(|number| number.to_string() == *value)
}

fn is_version_at_least(value: Option<&JsonValue>, minimum_minor: u16) -> bool {
    let Some(fields) = value.and_then(JsonValue::as_object) else {
        return false;
    };
    fields.get("major").and_then(JsonValue::as_u16) == Some(1)
        && fields
            .get("minor")
            .and_then(JsonValue::as_u16)
            .is_some_and(|minor| minor >= minimum_minor)
}

fn is_exact_schema_v1(value: Option<&JsonValue>, minor: u16) -> bool {
    let Some(fields) = value.and_then(JsonValue::as_object) else {
        return false;
    };
    fields.get("major").and_then(JsonValue::as_u16) == Some(1)
        && fields.get("minor").and_then(JsonValue::as_u16) == Some(minor)
}
