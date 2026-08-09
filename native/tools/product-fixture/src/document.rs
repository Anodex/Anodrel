//! The one document and semantic action this fixture ever submits.
//!
//! The document is a compile-time constant. The fixture reads no file, accepts
//! no argument, and has no way to render anything else, so a provisioned
//! fixture child cannot be turned into a general UI delivery tool.

/// The semantic action the host renders and the fixture waits for.
pub const FIXTURE_ACTION: &str = "fixture.session.action";

/// The exact `anodrel.ui.document.v1` value the fixture delivers.
pub const FIXTURE_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"fixture.root","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"fixture.eyebrow","kind":"text","value":"VERIFIED ANODREL PRODUCT SESSION","fontSize":14,"tone":"accent"},{"id":"fixture.title","kind":"text","value":"Signed child, authenticated window","fontSize":28,"tone":"primary"},{"id":"fixture.detail","kind":"text","value":"This document arrived from a child launched only after a machine-policy record, a locked digest check, and an Authenticode publisher match.","fontSize":16,"tone":"secondary"},{"id":"fixture.session.action","kind":"action","label":"Complete product session","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::{FIXTURE_ACTION, FIXTURE_DOCUMENT};

    #[test]
    fn the_fixture_document_is_a_strict_version_one_object() {
        let value = JsonValue::parse(FIXTURE_DOCUMENT).expect("the fixture document is JSON");
        let fields = value
            .as_object()
            .expect("the fixture document is an object");
        assert_eq!(
            fields.get("format").and_then(JsonValue::as_string),
            Some("anodrel.ui.document.v1")
        );
    }

    #[test]
    fn the_awaited_action_exists_in_the_delivered_document() {
        // The fixture would otherwise wait for an action the host can never
        // render, and its safe timeout would look like a lifecycle defect.
        assert!(FIXTURE_DOCUMENT.contains(&format!(r#""id":"{FIXTURE_ACTION}""#)));
    }

    #[test]
    fn the_fixture_document_decodes_through_the_hosts_own_codec() {
        // The host decodes this string with the strict v1 codec before it
        // renders anything. Failing here would surface at run time as a
        // rejected replacement inside a real product session.
        let document =
            anodrel_ui_document::decode(FIXTURE_DOCUMENT).expect("the fixture document is valid");
        assert_eq!(document.root().id().as_str(), "fixture.root");
    }

    #[test]
    fn the_fixture_document_stays_inside_the_operation_limit() {
        // `ui.document.replace` bounds its encoded document at 24 KiB inside the
        // 64 KiB wire message. A fixed fixture must never approach that edge.
        assert!(FIXTURE_DOCUMENT.len() < 4 * 1024);
    }
}
