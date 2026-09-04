//! Local strict validation for one `tray.replace` model.

use anodrel_json::JsonValue;

use crate::{UiClientError, context_menu_model::decode_context_menu_model};

/// The operation-level bound from the native tray contract.
pub const MAX_TRAY_MODEL_BYTES: usize = 8 * 1024;

/// Parses and validates one exact complete tray model before it crosses the
/// authenticated conversation. The host validates it again and retains its
/// icon, popup placement, commands, and callback route.
pub(crate) fn decode_tray_model(encoded: &str) -> Result<JsonValue, UiClientError> {
    if encoded.len() > MAX_TRAY_MODEL_BYTES {
        return Err(UiClientError::TrayInvalid);
    }
    decode_context_menu_model(encoded).map_err(|_| UiClientError::TrayInvalid)
}

#[cfg(test)]
mod tests {
    use super::{MAX_TRAY_MODEL_BYTES, decode_tray_model};
    use crate::UiClientError;

    #[test]
    fn accepts_only_one_strict_bounded_tray_model() {
        assert!(
            decode_tray_model(
                r#"{"items":[{"id":"window.open","label":"Open window","enabled":true}]}"#
            )
            .is_ok()
        );
        for invalid in [
            r#"{}"#,
            r#"{"items":[]}"#,
            r#"{"items":[{"id":"window.open","label":"Open window","enabled":true,"tooltip":"private"}]}"#,
        ] {
            assert_eq!(decode_tray_model(invalid), Err(UiClientError::TrayInvalid));
        }
    }

    #[test]
    fn rejects_an_oversized_model_before_a_request_is_possible() {
        let oversized = format!(
            r#"{{"items":[{{"id":"window.open","label":"{}","enabled":true}}]}}"#,
            "x".repeat(MAX_TRAY_MODEL_BYTES)
        );
        assert_eq!(
            decode_tray_model(&oversized),
            Err(UiClientError::TrayInvalid)
        );
    }
}
