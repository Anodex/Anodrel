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

/// Decodes the complete native menu model, including its optional local
/// shortcuts when the negotiated protocol version permits them.
pub(super) fn menu_replace_payload(
    value: &JsonValue,
    shortcuts_allowed: bool,
) -> Option<MenuModel> {
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
