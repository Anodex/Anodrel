//! The two fixed documents and fixed semantic actions for the structure probe.
//!
//! No document or action is supplied by a caller, so this child cannot become
//! a configurable event generator.

/// The only first-stage action this diagnostic accepts.
pub const PREPARE_ACTION: &str = "native.structure.prepare";

/// The only second-stage action this diagnostic accepts.
pub const COMPLETE_ACTION: &str = "native.structure.complete";

/// The initial document delivered through the authenticated session.
pub const INITIAL_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"native.structure.initial","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"native.structure.eyebrow","kind":"text","value":"ANODREL STRUCTURE EVENT DIAGNOSTIC","fontSize":14,"tone":"accent"},{"id":"native.structure.title","kind":"text","value":"Initial authenticated document","fontSize":28,"tone":"primary"},{"id":"native.structure.detail","kind":"text","value":"A fixed host-only accessibility listener is now ready for this document's one controlled replacement.","fontSize":16,"tone":"secondary"},{"id":"native.structure.prepare","kind":"action","label":"Prepare structure replacement","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

/// The replacement document delivered only after the fixed first action.
pub const REPLACEMENT_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"native.structure.replacement","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"native.structure.eyebrow","kind":"text","value":"ANODREL STRUCTURE EVENT DIAGNOSTIC","fontSize":14,"tone":"accent"},{"id":"native.structure.title","kind":"text","value":"Replacement authenticated document","fontSize":28,"tone":"primary"},{"id":"native.structure.detail","kind":"text","value":"The fixed root now represents this compiled replacement document.","fontSize":16,"tone":"secondary"},{"id":"native.structure.complete","kind":"action","label":"Complete structure diagnostic","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::{COMPLETE_ACTION, INITIAL_DOCUMENT, PREPARE_ACTION, REPLACEMENT_DOCUMENT};

    #[test]
    fn both_documents_are_strict_version_one_objects() {
        for document in [INITIAL_DOCUMENT, REPLACEMENT_DOCUMENT] {
            let value = JsonValue::parse(document).expect("the diagnostic document is JSON");
            let fields = value
                .as_object()
                .expect("the diagnostic document is an object");
            assert_eq!(
                fields.get("format").and_then(JsonValue::as_string),
                Some("anodrel.ui.document.v1")
            );
        }
    }

    #[test]
    fn each_stage_action_exists_only_in_its_compiled_document() {
        assert!(INITIAL_DOCUMENT.contains(&format!(r#""id":"{PREPARE_ACTION}""#)));
        assert!(!INITIAL_DOCUMENT.contains(&format!(r#""id":"{COMPLETE_ACTION}""#)));
        assert!(REPLACEMENT_DOCUMENT.contains(&format!(r#""id":"{COMPLETE_ACTION}""#)));
        assert!(!REPLACEMENT_DOCUMENT.contains(&format!(r#""id":"{PREPARE_ACTION}""#)));
    }

    #[test]
    fn both_documents_decode_through_the_host_codec() {
        for document in [INITIAL_DOCUMENT, REPLACEMENT_DOCUMENT] {
            anodrel_ui_document::decode(document)
                .expect("the diagnostic document remains valid for the host");
        }
    }

    #[test]
    fn both_documents_stay_well_inside_the_operation_limit() {
        assert!(INITIAL_DOCUMENT.len() < 4 * 1024);
        assert!(REPLACEMENT_DOCUMENT.len() < 4 * 1024);
    }
}
