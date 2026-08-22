//! Typed parsing for whole-surface native field snapshots.

use anodrel_json::JsonValue;
use anodrel_ui::{ElementId, Field, MAX_FIELD_LENGTH};
use anodrel_ui_session::MAX_SNAPSHOT_FIELDS;

use crate::UiClientError;

/// One current value from the session's host-owned form surface.
///
/// The value carries no edit history, focus, caret, selection, timestamp, or
/// native control. It is one entry from a deliberate whole-surface snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiFieldValue {
    id: ElementId,
    value: String,
}

impl UiFieldValue {
    /// Returns the validated current document element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the current host-owned field value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Every current field value from one authenticated session surface.
///
/// Entries remain in canonical element-ID order, which keeps this snapshot
/// independent of focus and edit history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiFieldSnapshot {
    fields: Vec<UiFieldValue>,
}

impl UiFieldSnapshot {
    /// Returns every current field value in canonical element-ID order.
    #[must_use]
    pub fn fields(&self) -> &[UiFieldValue] {
        &self.fields
    }

    pub(crate) fn parse(result: &JsonValue) -> Result<Self, UiClientError> {
        let object = result.as_object().ok_or(UiClientError::ResponseInvalid)?;
        if object.len() != 1 {
            return Err(UiClientError::ResponseInvalid);
        }
        let encoded_fields = object
            .get("fields")
            .and_then(as_array)
            .ok_or(UiClientError::ResponseInvalid)?;
        if encoded_fields.len() > MAX_SNAPSHOT_FIELDS {
            return Err(UiClientError::ResponseInvalid);
        }

        let mut fields = Vec::with_capacity(encoded_fields.len());
        let mut previous_id = None::<ElementId>;
        for encoded in encoded_fields {
            let object = encoded.as_object().ok_or(UiClientError::ResponseInvalid)?;
            if object.len() != 2 {
                return Err(UiClientError::ResponseInvalid);
            }
            let id = object
                .get("id")
                .and_then(JsonValue::as_string)
                .ok_or(UiClientError::ResponseInvalid)
                .and_then(|value| {
                    ElementId::new(value.to_owned()).map_err(|_| UiClientError::ResponseInvalid)
                })?;
            let value = object
                .get("value")
                .and_then(JsonValue::as_string)
                .ok_or(UiClientError::ResponseInvalid)?;
            Field::new(
                id.clone(),
                "Field",
                value.to_owned(),
                MAX_FIELD_LENGTH,
                14,
                true,
            )
            .map_err(|_| UiClientError::ResponseInvalid)?;
            if previous_id
                .as_ref()
                .is_some_and(|previous_id| id <= *previous_id)
            {
                return Err(UiClientError::ResponseInvalid);
            }
            previous_id = Some(id.clone());
            fields.push(UiFieldValue {
                id,
                value: value.to_owned(),
            });
        }
        Ok(Self { fields })
    }
}

fn as_array(value: &JsonValue) -> Option<&[JsonValue]> {
    match value {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }
}
