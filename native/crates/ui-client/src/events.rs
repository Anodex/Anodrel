//! Typed parsing for the bounded semantic-action result.

use anodrel_json::JsonValue;
use anodrel_ui::ElementId;

use crate::{DocumentRevision, UiClientError};

/// The host's maximum number of interaction candidates per pull.
pub const MAX_ACTIONS_PER_BATCH: usize = 32;

/// One current semantic action accepted by the host session.
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

/// One bounded `ui.events.read` result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiActionBatch {
    actions: Vec<UiAction>,
    dropped: u32,
    discarded: u32,
}

impl UiActionBatch {
    /// Returns the current accepted actions in host delivery order.
    #[must_use]
    pub fn actions(&self) -> &[UiAction] {
        &self.actions
    }

    /// Returns the number of candidates dropped because the host queue was full.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Returns the number of candidates discarded by revision/action validation.
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
        let mut actions = Vec::with_capacity(events.len());
        for event in events {
            actions.push(UiAction::parse(event)?);
        }
        Ok(Self {
            actions,
            dropped,
            discarded,
        })
    }
}

impl UiAction {
    fn parse(event: &JsonValue) -> Result<Self, UiClientError> {
        let fields = event.as_object().ok_or(UiClientError::ResponseInvalid)?;
        if fields.get("kind").and_then(JsonValue::as_string) != Some("event")
            || fields.get("eventName").and_then(JsonValue::as_string) != Some("ui.action.invoked")
            || fields.get("source").and_then(JsonValue::as_string) != Some("native.ui")
            || !is_version_at_least(fields.get("protocolVersion"), 2)
            || !is_exact_schema_v1(fields.get("schemaVersion"))
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
        Ok(Self {
            revision,
            action: action.to_owned(),
        })
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

fn is_exact_schema_v1(value: Option<&JsonValue>) -> bool {
    let Some(fields) = value.and_then(JsonValue::as_object) else {
        return false;
    };
    fields.get("major").and_then(JsonValue::as_u16) == Some(1)
        && fields.get("minor").and_then(JsonValue::as_u16) == Some(0)
}
