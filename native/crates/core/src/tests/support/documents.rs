//! Common encoded-document and response helpers for core tests.

use super::*;

pub(in crate::tests) fn ui_document_payload(document: &str) -> String {
    object([("document", JsonValue::String(document.to_owned()))]).to_json()
}

pub(in crate::tests) fn valid_ui_document(label: &str) -> String {
    format!(
        r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"root","kind":"action","label":"{label}","fontSize":16,"enabled":true,"tone":"accent"}}}}"#
    )
}

pub(in crate::tests) fn valid_ui_document_v2() -> &'static str {
    r#"{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"content","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}}"#
}

pub(in crate::tests) fn valid_ui_document_v3(value: &str, politeness: &str) -> String {
    format!(
        r#"{{"format":"anodrel.ui.document.v3","root":{{"id":"status","kind":"status","value":"{value}","fontSize":16,"tone":"accent","politeness":"{politeness}"}}}}"#
    )
}

pub(in crate::tests) fn field<'a>(value: &'a JsonValue, field: &str) -> &'a JsonValue {
    &value.as_object().expect("response is an object")[field]
}
