//! The one document and semantic action the native UI diagnostic submits.
//!
//! The value is compiled in. The diagnostic accepts no document, argument, or
//! configuration, so it cannot become a general UI delivery tool.

/// The one semantic action this diagnostic waits to receive.
pub const NATIVE_UI_ACTION: &str = "native.ui.complete";

/// The exact `anodrel.ui.document.v1` value sent through the authenticated
/// session.
pub const NATIVE_UI_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"native.ui.root","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"native.ui.eyebrow","kind":"text","value":"ANODREL NATIVE UI DIAGNOSTIC","fontSize":14,"tone":"accent"},{"id":"native.ui.title","kind":"text","value":"Compiled child, owned native session","fontSize":28,"tone":"primary"},{"id":"native.ui.detail","kind":"text","value":"This window was delivered, rendered, and closed through Anodrel without a browser or Node.js runtime.","fontSize":16,"tone":"secondary"},{"id":"native.ui.complete","kind":"action","label":"Complete native UI diagnostic","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::{NATIVE_UI_ACTION, NATIVE_UI_DOCUMENT};

    #[test]
    fn the_native_ui_document_is_a_strict_version_one_object() {
        let value = JsonValue::parse(NATIVE_UI_DOCUMENT).expect("the diagnostic document is JSON");
        let fields = value
            .as_object()
            .expect("the diagnostic document is an object");
        assert_eq!(
            fields.get("format").and_then(JsonValue::as_string),
            Some("anodrel.ui.document.v1")
        );
    }

    #[test]
    fn the_awaited_action_exists_in_the_delivered_document() {
        assert!(NATIVE_UI_DOCUMENT.contains(&format!(r#""id":"{NATIVE_UI_ACTION}""#)));
    }

    #[test]
    fn the_native_ui_document_decodes_through_the_host_codec() {
        let document = anodrel_ui_document::decode(NATIVE_UI_DOCUMENT)
            .expect("the diagnostic document is valid");
        assert_eq!(document.root().id().as_str(), "native.ui.root");
    }

    #[test]
    fn the_native_ui_document_stays_well_inside_the_operation_limit() {
        assert!(NATIVE_UI_DOCUMENT.len() < 4 * 1024);
    }
}
