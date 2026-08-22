//! Local strict validation for one `menu.replace` model.

use anodrel_json::JsonValue;
use anodrel_menu::{Menu, MenuAction, MenuActionId, MenuModel, MenuShortcut, MenuText};

use crate::UiClientError;

/// The stricter operation-level bound from the native menu contract.
pub const MAX_MENU_MODEL_BYTES: usize = 16 * 1024;

/// Parses and validates an exact complete menu model before it crosses the
/// authenticated conversation. The host validates it again and remains the
/// authority for its capability, revision, and native attachment.
pub(crate) fn decode_menu_model(encoded: &str) -> Result<JsonValue, UiClientError> {
    if encoded.len() > MAX_MENU_MODEL_BYTES {
        return Err(UiClientError::MenuInvalid);
    }
    let value = JsonValue::parse(encoded).map_err(|_| UiClientError::MenuInvalid)?;
    let fields = exact_object(&value, &["menus"])?;
    let JsonValue::Array(menus) = field(fields, "menus")? else {
        return Err(UiClientError::MenuInvalid);
    };
    let menus = menus
        .iter()
        .map(decode_menu)
        .collect::<Result<Vec<_>, _>>()?;
    MenuModel::new(menus).map_err(|_| UiClientError::MenuInvalid)?;
    Ok(value)
}

fn decode_menu(value: &JsonValue) -> Result<Menu, UiClientError> {
    let fields = exact_object(value, &["label", "items"])?;
    let label = string(fields, "label")?;
    let JsonValue::Array(items) = field(fields, "items")? else {
        return Err(UiClientError::MenuInvalid);
    };
    let items = items
        .iter()
        .map(decode_action)
        .collect::<Result<Vec<_>, _>>()?;
    Menu::new(
        MenuText::new(label).map_err(|_| UiClientError::MenuInvalid)?,
        items,
    )
    .map_err(|_| UiClientError::MenuInvalid)
}

fn decode_action(value: &JsonValue) -> Result<MenuAction, UiClientError> {
    let fields = value.as_object().ok_or(UiClientError::MenuInvalid)?;
    let has_shortcut = fields.contains_key("shortcut");
    if fields.len() != 3 + usize::from(has_shortcut)
        || ["id", "label", "enabled"]
            .iter()
            .any(|name| !fields.contains_key(*name))
    {
        return Err(UiClientError::MenuInvalid);
    }
    let id = MenuActionId::new(string(fields, "id")?).map_err(|_| UiClientError::MenuInvalid)?;
    let label = MenuText::new(string(fields, "label")?).map_err(|_| UiClientError::MenuInvalid)?;
    let JsonValue::Bool(enabled) = field(fields, "enabled")? else {
        return Err(UiClientError::MenuInvalid);
    };
    let action = MenuAction::new(id, label, *enabled);
    match fields.get("shortcut") {
        Some(JsonValue::String(shortcut)) => MenuShortcut::parse(shortcut)
            .map(|shortcut| action.with_shortcut(shortcut))
            .map_err(|_| UiClientError::MenuInvalid),
        Some(_) => Err(UiClientError::MenuInvalid),
        None => Ok(action),
    }
}

fn exact_object<'a>(
    value: &'a JsonValue,
    expected: &[&str],
) -> Result<&'a std::collections::BTreeMap<String, JsonValue>, UiClientError> {
    let fields = value.as_object().ok_or(UiClientError::MenuInvalid)?;
    if fields.len() != expected.len() || expected.iter().any(|name| !fields.contains_key(*name)) {
        return Err(UiClientError::MenuInvalid);
    }
    Ok(fields)
}

fn field<'a>(
    fields: &'a std::collections::BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a JsonValue, UiClientError> {
    fields.get(name).ok_or(UiClientError::MenuInvalid)
}

fn string<'a>(
    fields: &'a std::collections::BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, UiClientError> {
    field(fields, name)?
        .as_string()
        .ok_or(UiClientError::MenuInvalid)
}

#[cfg(test)]
mod tests {
    use super::{MAX_MENU_MODEL_BYTES, decode_menu_model};
    use crate::UiClientError;

    const MENU: &str = r#"{"menus":[{"label":"File","items":[{"id":"template.menu.complete","label":"Complete","enabled":true,"shortcut":"Ctrl+Shift+M"}]}]}"#;

    #[test]
    fn accepts_only_one_strict_bounded_complete_menu_model() {
        assert!(decode_menu_model(MENU).is_ok());
        for invalid in [
            r#"{}"#,
            r#"{"menus":[]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"complete","label":"Complete","enabled":true,"extra":false}]}]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"complete","label":"Complete","enabled":"true"}]}]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"complete","label":"Complete","enabled":true,"shortcut":"Ctrl+m"}]}]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"complete","label":"Complete","enabled":true,"shortcut":true}]}]}"#,
            r#"{"menus":[{"label":"File","items":[{"id":"complete.primary","label":"Primary","enabled":true,"shortcut":"Ctrl+M"},{"id":"complete.secondary","label":"Secondary","enabled":false,"shortcut":"Ctrl+M"}]}]}"#,
        ] {
            assert_eq!(decode_menu_model(invalid), Err(UiClientError::MenuInvalid));
        }
    }

    #[test]
    fn rejects_an_oversized_model_before_a_request_is_possible() {
        let oversized = format!(
            r#"{{"menus":[{{"label":"File","items":[{{"id":"complete","label":"{}","enabled":true}}]}}]}}"#,
            "x".repeat(MAX_MENU_MODEL_BYTES)
        );
        assert_eq!(
            decode_menu_model(&oversized),
            Err(UiClientError::MenuInvalid)
        );
    }
}
