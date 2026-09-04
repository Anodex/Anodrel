//! Exact decoding shared by host-owned semantic popup surfaces.

use super::*;

/// Decodes the common complete semantic action model within one route's bound.
pub(super) fn replace_payload(value: &JsonValue, max_bytes: usize) -> Option<ContextMenuModel> {
    if value.to_json().len() > max_bytes {
        return None;
    }
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    let JsonValue::Array(items) = fields.get("items")? else {
        return None;
    };
    let items = items
        .iter()
        .map(|item| {
            let fields = item.as_object()?;
            if fields.len() != 3 {
                return None;
            }
            let id = MenuActionId::new(fields.get("id")?.as_string()?.to_owned()).ok()?;
            let label = MenuText::new(fields.get("label")?.as_string()?.to_owned()).ok()?;
            let JsonValue::Bool(enabled) = fields.get("enabled")? else {
                return None;
            };
            Some(MenuAction::new(id, label, *enabled))
        })
        .collect::<Option<Vec<_>>>()?;
    ContextMenuModel::new(items).ok()
}
