//! The two fixed version-3 documents and actions for the live-status probe.
//!
//! No document or action is supplied by a caller, so this child cannot become
//! a configurable accessibility-event generator.

/// The only first-stage action this diagnostic accepts.
pub const PREPARE_ACTION: &str = "native.live.prepare";

/// The only second-stage action this diagnostic accepts.
pub const COMPLETE_ACTION: &str = "native.live.complete";

/// The first document, with the visible polite status under its fixed ID.
pub const INITIAL_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v3","root":{"id":"native.live.root","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"native.live.eyebrow","kind":"text","value":"ANODREL LIVE-STATUS EVENT DIAGNOSTIC","fontSize":14,"tone":"accent"},{"id":"native.live.title","kind":"text","value":"Polite result is ready","fontSize":28,"tone":"primary"},{"id":"native.live.status","kind":"status","value":"Ready to publish a visible result.","fontSize":18,"tone":"accent","politeness":"polite"},{"id":"native.live.prepare","kind":"action","label":"Publish changed live status","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

/// The second document, changing the same visible status to an assertive result.
pub const REPLACEMENT_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v3","root":{"id":"native.live.root","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"native.live.eyebrow","kind":"text","value":"ANODREL LIVE-STATUS EVENT DIAGNOSTIC","fontSize":14,"tone":"accent"},{"id":"native.live.title","kind":"text","value":"Changed result was published","fontSize":28,"tone":"primary"},{"id":"native.live.status","kind":"status","value":"Verification requires attention.","fontSize":18,"tone":"accent","politeness":"assertive"},{"id":"native.live.complete","kind":"action","label":"Complete live-status diagnostic","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::{COMPLETE_ACTION, INITIAL_DOCUMENT, PREPARE_ACTION, REPLACEMENT_DOCUMENT};

    #[test]
    fn both_documents_are_strict_version_three_objects() {
        for document in [INITIAL_DOCUMENT, REPLACEMENT_DOCUMENT] {
            let value = JsonValue::parse(document).expect("the diagnostic document is JSON");
            let fields = value
                .as_object()
                .expect("the diagnostic document is an object");
            assert_eq!(
                fields.get("format").and_then(JsonValue::as_string),
                Some("anodrel.ui.document.v3")
            );
            anodrel_ui_document::decode_v3(document)
                .expect("the diagnostic document remains valid for the host");
        }
    }

    #[test]
    fn replacement_changes_one_stable_visible_status() {
        let initial = anodrel_ui_document::decode_v3(INITIAL_DOCUMENT)
            .expect("initial diagnostic document is valid");
        let replacement = anodrel_ui_document::decode_v3(REPLACEMENT_DOCUMENT)
            .expect("replacement diagnostic document is valid");
        assert_eq!(
            initial.status().map(|status| status.id().as_str()),
            Some("native.live.status")
        );
        assert_eq!(
            replacement.status().map(|status| status.id().as_str()),
            Some("native.live.status")
        );
        assert_ne!(
            initial.status().map(|status| status.value()),
            replacement.status().map(|status| status.value())
        );
    }

    #[test]
    fn each_stage_action_exists_only_in_its_compiled_document() {
        assert!(INITIAL_DOCUMENT.contains(&format!(r#""id":"{PREPARE_ACTION}""#)));
        assert!(!INITIAL_DOCUMENT.contains(&format!(r#""id":"{COMPLETE_ACTION}""#)));
        assert!(REPLACEMENT_DOCUMENT.contains(&format!(r#""id":"{COMPLETE_ACTION}""#)));
        assert!(!REPLACEMENT_DOCUMENT.contains(&format!(r#""id":"{PREPARE_ACTION}""#)));
    }

    #[test]
    fn both_documents_stay_well_inside_the_operation_limit() {
        assert!(INITIAL_DOCUMENT.len() < 4 * 1024);
        assert!(REPLACEMENT_DOCUMENT.len() < 4 * 1024);
    }
}
