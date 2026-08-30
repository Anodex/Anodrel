//! Local strict validation for one `menu.context.replace` model.

use anodrel_json::JsonValue;
use anodrel_menu::{ContextMenuModel, MenuAction, MenuActionId, MenuText};

use crate::UiClientError;

/// The operation-level bound from the native context-menu contract.
pub const MAX_CONTEXT_MENU_MODEL_BYTES: usize = 8 * 1024;

/// Parses and validates one exact complete context-menu model before it crosses
/// the authenticated conversation. The host validates it again and retains all
/// native placement, command, and popup behavior.
pub(crate) fn decode_context_menu_model(encoded: &str) -> Result<JsonValue, UiClientError> {
    if encoded.len() > MAX_CONTEXT_MENU_MODEL_BYTES {
        return Err(UiClientError::ContextMenuInvalid);
    }
    let value = JsonValue::parse(encoded).map_err(|_| UiClientError::ContextMenuInvalid)?;
    let fields = exact_object(&value, &["items"])?;
    let JsonValue::Array(items) = field(fields, "items")? else {
        return Err(UiClientError::ContextMenuInvalid);
    };
    let items = items
        .iter()
        .map(decode_action)
        .collect::<Result<Vec<_>, _>>()?;
    ContextMenuModel::new(items).map_err(|_| UiClientError::ContextMenuInvalid)?;
    Ok(value)
}

fn decode_action(value: &JsonValue) -> Result<MenuAction, UiClientError> {
    let fields = exact_object(value, &["id", "label", "enabled"])?;
    let id =
        MenuActionId::new(string(fields, "id")?).map_err(|_| UiClientError::ContextMenuInvalid)?;
    let label =
        MenuText::new(string(fields, "label")?).map_err(|_| UiClientError::ContextMenuInvalid)?;
    let JsonValue::Bool(enabled) = field(fields, "enabled")? else {
        return Err(UiClientError::ContextMenuInvalid);
    };
    Ok(MenuAction::new(id, label, *enabled))
}

fn exact_object<'a>(
    value: &'a JsonValue,
    expected: &[&str],
) -> Result<&'a std::collections::BTreeMap<String, JsonValue>, UiClientError> {
    let fields = value.as_object().ok_or(UiClientError::ContextMenuInvalid)?;
    if fields.len() != expected.len() || expected.iter().any(|name| !fields.contains_key(*name)) {
        return Err(UiClientError::ContextMenuInvalid);
    }
    Ok(fields)
}

fn field<'a>(
    fields: &'a std::collections::BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a JsonValue, UiClientError> {
    fields.get(name).ok_or(UiClientError::ContextMenuInvalid)
}

fn string<'a>(
    fields: &'a std::collections::BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, UiClientError> {
    field(fields, name)?
        .as_string()
        .ok_or(UiClientError::ContextMenuInvalid)
}

#[cfg(test)]
mod tests {
    use super::{MAX_CONTEXT_MENU_MODEL_BYTES, decode_context_menu_model};
    use crate::UiClientError;

    const CONTEXT_MENU: &str =
        r#"{"items":[{"id":"template.context.complete","label":"Complete","enabled":true}]}"#;

    #[test]
    fn accepts_only_one_strict_bounded_context_menu_model() {
        assert!(decode_context_menu_model(CONTEXT_MENU).is_ok());
        for invalid in [
            r#"{}"#,
            r#"{"items":[]}"#,
            r#"{"items":[{"id":"complete","label":"Complete","enabled":true,"shortcut":"Ctrl+M"}]}"#,
            r#"{"items":[{"id":"complete","label":"Complete","enabled":"true"}]}"#,
            r#"{"items":[{"id":"same","label":"One","enabled":true},{"id":"same","label":"Two","enabled":false}]}"#,
        ] {
            assert_eq!(
                decode_context_menu_model(invalid),
                Err(UiClientError::ContextMenuInvalid)
            );
        }
    }

    #[test]
    fn rejects_an_oversized_model_before_a_request_is_possible() {
        let oversized = format!(
            r#"{{"items":[{{"id":"complete","label":"{}","enabled":true}}]}}"#,
            "x".repeat(MAX_CONTEXT_MENU_MODEL_BYTES)
        );
        assert_eq!(
            decode_context_menu_model(&oversized),
            Err(UiClientError::ContextMenuInvalid)
        );
    }
}
