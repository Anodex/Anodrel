//! Typed event values for host-owned context-menu and tray surfaces.

use anodrel_json::JsonValue;
use anodrel_menu::MenuActionId;

use crate::events::{UiEvent, is_exact_schema_v1, is_version_at_least};
use crate::{ContextMenuRevision, TrayRevision, UiClientError, events::UiEventBatch};

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

    pub(crate) fn parse(event: &JsonValue) -> Result<Self, UiClientError> {
        let (revision, action) = parse_popup_event(
            event,
            "menu.context.action.invoked",
            "native.context_menu",
            32,
            32,
            "contextMenuRevision",
            ContextMenuRevision::parse,
        )?;
        Ok(Self { revision, action })
    }
}

/// One current host-owned notification-area tray action accepted by the session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiTrayAction {
    revision: TrayRevision,
    action: MenuActionId,
}

impl UiTrayAction {
    /// Returns the exact complete tray revision that produced this action.
    #[must_use]
    pub const fn revision(&self) -> TrayRevision {
        self.revision
    }

    /// Returns the validated semantic tray action ID.
    #[must_use]
    pub fn action(&self) -> &str {
        self.action.as_str()
    }

    pub(crate) fn parse(event: &JsonValue) -> Result<Self, UiClientError> {
        let (revision, action) = parse_popup_event(
            event,
            "tray.action.invoked",
            "native.tray",
            33,
            33,
            "trayRevision",
            TrayRevision::parse,
        )?;
        Ok(Self { revision, action })
    }
}

/// One bounded context-menu-only `ui.events.read` result.
///
/// This preserves the ordinary document, menu, and tray readers: none silently
/// accepts another host-owned popup event kind.
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
        let (events, dropped, discarded) = UiEventBatch::parse(result)?.into_parts();
        let actions = events
            .into_iter()
            .map(|event| match event {
                UiEvent::ContextMenuAction(action) => Ok(action),
                UiEvent::DocumentAction(_) | UiEvent::MenuAction(_) | UiEvent::TrayAction(_) => {
                    Err(UiClientError::ResponseInvalid)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            actions,
            dropped,
            discarded,
        })
    }
}

/// One bounded tray-only `ui.events.read` result.
///
/// This is separate from every other event reader so a caller cannot mistake a
/// document or menu event for a host-owned notification-area action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiTrayActionBatch {
    actions: Vec<UiTrayAction>,
    dropped: u32,
    discarded: u32,
}

impl UiTrayActionBatch {
    /// Returns current tray actions in host delivery order.
    #[must_use]
    pub fn actions(&self) -> &[UiTrayAction] {
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
        let (events, dropped, discarded) = UiEventBatch::parse(result)?.into_parts();
        let actions = events
            .into_iter()
            .map(|event| match event {
                UiEvent::TrayAction(action) => Ok(action),
                UiEvent::DocumentAction(_)
                | UiEvent::MenuAction(_)
                | UiEvent::ContextMenuAction(_) => Err(UiClientError::ResponseInvalid),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            actions,
            dropped,
            discarded,
        })
    }
}

fn parse_popup_event<Revision>(
    event: &JsonValue,
    event_name: &str,
    source: &str,
    minimum_minor: u16,
    schema_minor: u16,
    revision_field: &str,
    parse_revision: impl FnOnce(&str) -> Result<Revision, UiClientError>,
) -> Result<(Revision, MenuActionId), UiClientError> {
    let fields = event.as_object().ok_or(UiClientError::ResponseInvalid)?;
    if fields.get("kind").and_then(JsonValue::as_string) != Some("event")
        || fields.get("eventName").and_then(JsonValue::as_string) != Some(event_name)
        || fields.get("source").and_then(JsonValue::as_string) != Some(source)
        || !is_version_at_least(fields.get("protocolVersion"), minimum_minor)
        || !is_exact_schema_v1(fields.get("schemaVersion"), schema_minor)
    {
        return Err(UiClientError::ResponseInvalid);
    }
    let payload = fields
        .get("payload")
        .and_then(JsonValue::as_object)
        .ok_or(UiClientError::ResponseInvalid)?;
    let revision = payload
        .get(revision_field)
        .and_then(JsonValue::as_string)
        .ok_or(UiClientError::ResponseInvalid)
        .and_then(parse_revision)?;
    let action = payload
        .get("action")
        .and_then(JsonValue::as_string)
        .ok_or(UiClientError::ResponseInvalid)?;
    let action =
        MenuActionId::new(action.to_owned()).map_err(|_| UiClientError::ResponseInvalid)?;
    Ok((revision, action))
}
