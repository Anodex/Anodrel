//! Composition of the one machine-policy record this helper may write.
//!
//! The record shape is defined by `docs/LAUNCH.md`. This module builds it from
//! measured facts only — a recomputed executable digest and an Authenticode
//! leaf fingerprint Windows actually accepted — and then re-validates the result
//! through the same parser the native host uses. A record the helper could not
//! itself validate is never written.

use std::{
    fmt,
    fs::File,
    io,
    path::{Path, PathBuf},
};

use anodrel_application::{
    InstalledApplication, InstalledApplicationError, MAX_EXECUTABLE_BYTES, sha256,
};
use anodrel_json::JsonValue;
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::fixture;

/// Builds and validates the fixture's record for one staged package root.
pub fn compose(package_root: &Path, executable: &Path) -> Result<String, RecordError> {
    let package_root = std::fs::canonicalize(package_root).map_err(RecordError::Io)?;
    let digest = executable_digest(executable)?;
    let signer = verify_embedded_signature(executable).map_err(RecordError::Signature)?;
    let record = render(
        &package_root,
        &digest,
        &sha256::to_lower_hex(&signer.as_bytes()),
    );

    // Fail before writing rather than leaving a machine-policy value the host
    // would reject at launch time.
    InstalledApplication::load_from_trusted_record(&record, fixture::APPLICATION_ID)
        .map_err(RecordError::Record)?;
    Ok(record)
}

fn executable_digest(executable: &Path) -> Result<String, RecordError> {
    let mut file = File::open(executable).map_err(RecordError::Io)?;
    let (digest, _) = sha256::digest_reader_limited(&mut file, MAX_EXECUTABLE_BYTES)
        .map_err(RecordError::Io)?
        .ok_or(RecordError::ExecutableTooLarge)?;
    Ok(sha256::to_lower_hex(&digest))
}

/// Renders the strict record JSON for measured facts.
fn render(package_root: &Path, executable_digest: &str, publisher_digest: &str) -> String {
    let capabilities = fixture::CAPABILITIES
        .iter()
        .map(|capability| JsonValue::String((*capability).to_owned()))
        .collect();
    JsonValue::Object(
        [
            (
                "recordVersion".to_owned(),
                object([
                    ("major", JsonValue::Number("1".to_owned())),
                    ("minor", JsonValue::Number("2".to_owned())),
                ]),
            ),
            (
                "applicationId".to_owned(),
                JsonValue::String(fixture::APPLICATION_ID.to_owned()),
            ),
            (
                "packageRoot".to_owned(),
                JsonValue::String(package_root.display().to_string()),
            ),
            (
                "executable".to_owned(),
                object([
                    (
                        "path",
                        JsonValue::String(fixture::EXECUTABLE_PATH.to_owned()),
                    ),
                    ("sha256", JsonValue::String(executable_digest.to_owned())),
                ]),
            ),
            (
                "publisher".to_owned(),
                object([(
                    "leafCertificateSha256",
                    JsonValue::String(publisher_digest.to_owned()),
                )]),
            ),
            ("capabilities".to_owned(), JsonValue::Array(capabilities)),
        ]
        .into_iter()
        .collect(),
    )
    .to_json()
}

fn object<const N: usize>(fields: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

/// The canonical path a caller supplied, kept private to this helper.
pub fn canonical_package_root(value: &str) -> io::Result<PathBuf> {
    let path = std::fs::canonicalize(value)?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the fixture package root is not a directory",
        ))
    }
}

/// A safe failure category while composing the fixture record.
#[derive(Debug)]
pub enum RecordError {
    Io(io::Error),
    ExecutableTooLarge,
    Signature(SignatureError),
    Record(InstalledApplicationError),
}

impl fmt::Display for RecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io(_) => "the staged fixture executable could not be read",
            Self::ExecutableTooLarge => "the staged fixture executable exceeds its limit",
            Self::Signature(_) => "Windows did not accept the fixture executable signature",
            Self::Record(_) => "the composed fixture record did not validate",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RecordError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Signature(error) => Some(error),
            Self::Record(error) => Some(error),
            Self::ExecutableTooLarge => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::{fixture, render};

    const DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const FINGERPRINT: &str = "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585";

    #[test]
    fn a_composed_record_carries_exactly_the_documented_version_and_fields() {
        let record = render(std::path::Path::new("C:\\fixture"), DIGEST, FINGERPRINT);
        let value = JsonValue::parse(&record).expect("the composed record is JSON");
        let fields = value.as_object().expect("the composed record is an object");

        let mut names = fields.keys().cloned().collect::<Vec<_>>();
        names.sort();
        assert_eq!(
            names,
            [
                "applicationId",
                "capabilities",
                "executable",
                "packageRoot",
                "publisher",
                "recordVersion",
            ]
        );
        assert_eq!(
            fields.get("applicationId").and_then(JsonValue::as_string),
            Some(fixture::APPLICATION_ID)
        );
    }

    #[test]
    fn a_composed_record_grants_only_the_fixtures_three_capabilities() {
        let record = render(std::path::Path::new("C:\\fixture"), DIGEST, FINGERPRINT);
        let value = JsonValue::parse(&record).expect("the composed record is JSON");
        let Some(JsonValue::Array(capabilities)) = value
            .as_object()
            .and_then(|fields| fields.get("capabilities"))
        else {
            panic!("the composed record has a capability array");
        };

        let granted = capabilities
            .iter()
            .filter_map(JsonValue::as_string)
            .collect::<Vec<_>>();
        assert_eq!(granted, fixture::CAPABILITIES);
    }

    #[test]
    fn a_composed_record_stays_inside_the_record_size_limit() {
        let record = render(
            std::path::Path::new(&format!("C:\\{}", "fixture".repeat(30))),
            DIGEST,
            FINGERPRINT,
        );
        assert!(record.len() < anodrel_application::MAX_INSTALL_RECORD_BYTES);
    }
}
