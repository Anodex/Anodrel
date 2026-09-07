//! Strict JSON-field and contained-path helpers shared by manifest sections.

use std::collections::BTreeMap;

use anodrel_json::JsonValue;

use crate::ReleaseManifestError;

pub(super) fn exact_fields(
    fields: &BTreeMap<String, JsonValue>,
    expected: &[&str],
) -> Result<(), ReleaseManifestError> {
    (fields.len() == expected.len() && expected.iter().all(|name| fields.contains_key(*name)))
        .then_some(())
        .ok_or(ReleaseManifestError::Invalid)
}

pub(super) fn required_object<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, ReleaseManifestError> {
    fields
        .get(name)
        .and_then(JsonValue::as_object)
        .ok_or(ReleaseManifestError::Invalid)
}

pub(super) fn required_string<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, ReleaseManifestError> {
    fields
        .get(name)
        .and_then(JsonValue::as_string)
        .ok_or(ReleaseManifestError::Invalid)
}

pub(super) fn required_u16(
    fields: &BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<u16, ReleaseManifestError> {
    fields
        .get(name)
        .and_then(JsonValue::as_u16)
        .ok_or(ReleaseManifestError::Invalid)
}

pub(super) fn required_u64(
    fields: &BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<u64, ReleaseManifestError> {
    let Some(JsonValue::Number(value)) = fields.get(name) else {
        return Err(ReleaseManifestError::PayloadInvalid);
    };
    if value.starts_with('-') || value.contains(['.', 'e', 'E']) {
        return Err(ReleaseManifestError::PayloadInvalid);
    }
    value
        .parse()
        .map_err(|_| ReleaseManifestError::PayloadInvalid)
}

pub(super) fn is_valid_executable_path(path: &str) -> bool {
    !path.is_empty()
        && path.to_ascii_lowercase().ends_with(".exe")
        && !path.contains(['\\', ':'])
        && path.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.chars().any(char::is_control)
        })
}
