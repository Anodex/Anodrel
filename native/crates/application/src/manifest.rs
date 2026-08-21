//! Strict manifest parsing for the application package contract.

use std::collections::BTreeMap;

use anodrel_json::JsonValue;

use crate::{ApplicationError, TEXT_CONTENT_FORMAT, sha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationIdentity {
    application_id: String,
    display_name: String,
}

impl ApplicationIdentity {
    pub fn application_id(&self) -> &str {
        &self.application_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationManifest {
    identity: ApplicationIdentity,
    content_path: String,
    content_digest: [u8; 32],
}

impl ApplicationManifest {
    pub fn parse(input: &str) -> Result<Self, ApplicationError> {
        let root = JsonValue::parse(input).map_err(|_| ApplicationError::InvalidManifest)?;
        let fields = root.as_object().ok_or(ApplicationError::InvalidManifest)?;
        exact_fields(
            fields,
            &["manifestVersion", "applicationId", "displayName", "content"],
        )?;
        validate_version(required_object(fields, "manifestVersion")?)?;

        let application_id = required_string(fields, "applicationId")?;
        let display_name = required_string(fields, "displayName")?;
        if !is_valid_application_id(application_id) || !is_valid_display_name(display_name) {
            return Err(ApplicationError::InvalidManifest);
        }

        let content = required_object(fields, "content")?;
        exact_fields(content, &["format", "path", "sha256"])?;
        if required_string(content, "format")? != TEXT_CONTENT_FORMAT {
            return Err(ApplicationError::InvalidManifest);
        }

        let content_path = required_string(content, "path")?;
        if !is_valid_content_path(content_path) {
            return Err(ApplicationError::InvalidContentPath);
        }
        let content_digest = sha256::parse_lower_hex(required_string(content, "sha256")?)
            .ok_or(ApplicationError::InvalidManifest)?;

        Ok(Self {
            identity: ApplicationIdentity {
                application_id: application_id.to_owned(),
                display_name: display_name.to_owned(),
            },
            content_path: content_path.to_owned(),
            content_digest,
        })
    }

    pub fn identity(&self) -> &ApplicationIdentity {
        &self.identity
    }

    /// Returns the package-relative content path exactly as the manifest
    /// declared it.
    ///
    /// This is the declared path, never the canonical filesystem path. A host
    /// surface may display it; the resolved absolute path stays internal.
    pub fn content_path(&self) -> &str {
        &self.content_path
    }

    pub(crate) fn content_digest(&self) -> &[u8; 32] {
        &self.content_digest
    }
}

fn exact_fields(
    fields: &BTreeMap<String, JsonValue>,
    expected: &[&str],
) -> Result<(), ApplicationError> {
    if fields.len() != expected.len() || expected.iter().any(|field| !fields.contains_key(*field)) {
        return Err(ApplicationError::InvalidManifest);
    }
    Ok(())
}

fn required_string<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<&'a str, ApplicationError> {
    fields
        .get(field)
        .and_then(JsonValue::as_string)
        .ok_or(ApplicationError::InvalidManifest)
}

fn required_object<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, ApplicationError> {
    fields
        .get(field)
        .and_then(JsonValue::as_object)
        .ok_or(ApplicationError::InvalidManifest)
}

fn validate_version(fields: &BTreeMap<String, JsonValue>) -> Result<(), ApplicationError> {
    exact_fields(fields, &["major", "minor"])?;
    let major = fields
        .get("major")
        .and_then(JsonValue::as_u16)
        .ok_or(ApplicationError::InvalidManifest)?;
    let minor = fields
        .get("minor")
        .and_then(JsonValue::as_u16)
        .ok_or(ApplicationError::InvalidManifest)?;
    if major == 1 && minor == 0 {
        Ok(())
    } else {
        Err(ApplicationError::InvalidManifest)
    }
}

/// Returns whether a value uses the stable application-ID grammar.
#[must_use]
pub fn is_valid_application_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !(3..=128).contains(&bytes.len()) {
        return false;
    }
    matches!(bytes.first(), Some(b'a'..=b'z' | b'0'..=b'9'))
        && matches!(bytes.last(), Some(b'a'..=b'z' | b'0'..=b'9'))
        && bytes
            .iter()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_'))
}

/// Returns whether a display name satisfies manifest version 1.0's bounds.
#[must_use]
pub fn is_valid_display_name(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 80 && !value.chars().any(char::is_control)
}

fn is_valid_content_path(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 240
        || value.contains(['\\', ':'])
        || value.as_bytes().contains(&0)
    {
        return false;
    }

    value
        .split('/')
        .all(|component| !component.is_empty() && component != "." && component != "..")
}

#[cfg(test)]
mod tests {
    use super::ApplicationManifest;
    use crate::ApplicationError;

    const DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn manifest(application_id: &str, path: &str) -> String {
        format!(
            r#"{{
                "manifestVersion": {{"major": 1, "minor": 0}},
                "applicationId": "{application_id}",
                "displayName": "Anodrel Sample",
                "content": {{
                    "format": "anodrel.text.v1",
                    "path": "{path}",
                    "sha256": "{DIGEST}"
                }}
            }}"#
        )
    }

    #[test]
    fn parses_a_valid_manifest() {
        let manifest =
            ApplicationManifest::parse(&manifest("org.anodrel.sample", "content/main.txt"))
                .expect("manifest is valid");
        assert_eq!(manifest.identity().application_id(), "org.anodrel.sample");
        assert_eq!(manifest.identity().display_name(), "Anodrel Sample");
    }

    #[test]
    fn rejects_unknown_fields_and_invalid_identities() {
        let unknown = manifest("org.anodrel.sample", "content/main.txt")
            .replace(r#""content": {"#, r#""unexpected": true, "content": {"#);
        assert!(matches!(
            ApplicationManifest::parse(&unknown),
            Err(ApplicationError::InvalidManifest)
        ));
        assert!(matches!(
            ApplicationManifest::parse(&manifest("Org.Anodrel.Sample", "content/main.txt")),
            Err(ApplicationError::InvalidManifest)
        ));
    }

    #[test]
    fn rejects_paths_that_could_escape_the_package() {
        for path in [
            "../outside.txt",
            "./content.txt",
            "content\\\\main.txt",
            "C:/content.txt",
        ] {
            assert!(matches!(
                ApplicationManifest::parse(&manifest("org.anodrel.sample", path)),
                Err(ApplicationError::InvalidContentPath)
            ));
        }
    }
}
