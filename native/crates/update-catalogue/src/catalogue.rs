//! Exact catalogue parsing and opaque installer-byte comparisons.

use std::collections::BTreeMap;

use anodrel_application::{is_valid_application_id, sha256};
use anodrel_json::JsonValue;
use anodrel_network::NetworkOrigin;
use anodrel_windows_installer::{PackageVersion, ReleaseManifest};

use crate::{MAX_UPDATE_CATALOGUE_BYTES, MAX_UPDATE_IMAGE_BYTES, UpdateCatalogueError};

/// One strict update candidate selected by a separately signed catalogue.
pub struct UpdateCatalogue {
    application_id: String,
    package_version: PackageVersion,
    publisher_fingerprint: [u8; 32],
    installer: UpdateInstaller,
}

impl UpdateCatalogue {
    /// Parses one exact version-1 UTF-8 update catalogue.
    pub fn parse(input: &str) -> Result<Self, UpdateCatalogueError> {
        if input.len() > MAX_UPDATE_CATALOGUE_BYTES {
            return Err(UpdateCatalogueError::TooLarge);
        }
        let root = JsonValue::parse(input).map_err(|_| UpdateCatalogueError::Invalid)?;
        let fields = root.as_object().ok_or(UpdateCatalogueError::Invalid)?;
        exact_fields(
            fields,
            &[
                "formatVersion",
                "applicationId",
                "packageVersion",
                "publisher",
                "installer",
            ],
        )?;
        parse_format_version(required_object(fields, "formatVersion")?)?;
        let application_id = required_string(fields, "applicationId")?;
        if !is_valid_application_id(application_id) {
            return Err(UpdateCatalogueError::Invalid);
        }
        let package_version = parse_package_version(required_object(fields, "packageVersion")?)?;
        let publisher_fingerprint = parse_publisher(required_object(fields, "publisher")?)?;
        let installer = UpdateInstaller::parse(required_object(fields, "installer")?)?;
        Ok(Self {
            application_id: application_id.to_owned(),
            package_version,
            publisher_fingerprint,
            installer,
        })
    }

    /// Returns the catalogue's signed application identity.
    #[must_use]
    pub fn application_id(&self) -> &str {
        &self.application_id
    }

    /// Returns the candidate's declared release version.
    #[must_use]
    pub const fn package_version(&self) -> PackageVersion {
        self.package_version
    }

    /// Checks the candidate against a host-held installed identity and signer.
    #[must_use]
    pub fn matches_installed(&self, application_id: &str, publisher: [u8; 32]) -> bool {
        self.application_id == application_id && self.publisher_fingerprint == publisher
    }

    /// Checks one locked installer release against this signed candidate.
    ///
    /// The caller must first establish that the release came from an accepted
    /// Authenticode image. This comparison has no signature, file, process, or
    /// network operation of its own.
    #[must_use]
    pub fn matches_release(&self, release: &ReleaseManifest) -> bool {
        self.application_id == release.application_id()
            && self.package_version == release.package_version()
            && release.matches_publisher_fingerprint(self.publisher_fingerprint)
    }

    /// Returns whether this catalogue is strictly newer than an installed version.
    #[must_use]
    pub fn is_newer_than(&self, installed: PackageVersion) -> bool {
        self.package_version > installed
    }

    /// Returns the one exact installer retrieval and byte-check contract.
    #[must_use]
    pub fn installer(&self) -> &UpdateInstaller {
        &self.installer
    }
}

/// One exact HTTPS installer retrieval and opaque byte descriptor.
pub struct UpdateInstaller {
    origin: NetworkOrigin,
    request_path: String,
    byte_length: u64,
    digest: [u8; 32],
}

impl UpdateInstaller {
    fn parse(fields: &BTreeMap<String, JsonValue>) -> Result<Self, UpdateCatalogueError> {
        exact_fields(fields, &["origin", "path", "byteLength", "sha256"])?;
        let origin = parse_origin(required_object(fields, "origin")?)?;
        let request_path = required_string(fields, "path")?;
        if !is_valid_request_path(request_path) {
            return Err(UpdateCatalogueError::InstallerLocationInvalid);
        }
        let byte_length = required_u64(fields, "byteLength")?;
        if !(1..=MAX_UPDATE_IMAGE_BYTES).contains(&byte_length) {
            return Err(UpdateCatalogueError::InstallerBytesInvalid);
        }
        let digest = sha256::parse_lower_hex(required_string(fields, "sha256")?)
            .ok_or(UpdateCatalogueError::InstallerBytesInvalid)?;
        Ok(Self {
            origin,
            request_path: request_path.to_owned(),
            byte_length,
            digest,
        })
    }

    /// Returns the exact TLS origin selected by the signed catalogue.
    #[must_use]
    pub fn origin(&self) -> &NetworkOrigin {
        &self.origin
    }

    /// Returns the exact relative request path selected by the signed catalogue.
    #[must_use]
    pub fn request_path(&self) -> &str {
        &self.request_path
    }

    /// Returns the exact expected installer-image byte length.
    #[must_use]
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// Checks one downloaded image's bytes without exposing its expected digest.
    #[must_use]
    pub fn matches_bytes(&self, actual: &[u8]) -> bool {
        self.matches_descriptor(actual.len() as u64, sha256::digest(actual))
    }

    /// Checks a caller-calculated downloaded-image descriptor without exposing
    /// the catalogue's expected digest.
    #[must_use]
    pub fn matches_descriptor(&self, byte_length: u64, digest: [u8; 32]) -> bool {
        byte_length == self.byte_length && digest == self.digest
    }
}

impl std::fmt::Debug for UpdateCatalogue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UpdateCatalogue")
            .field("application_id", &self.application_id)
            .field("package_version", &self.package_version)
            .field("publisher_fingerprint", &"[redacted]")
            .field("installer", &self.installer)
            .finish()
    }
}

impl std::fmt::Debug for UpdateInstaller {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UpdateInstaller")
            .field("origin", &self.origin)
            .field("request_path", &self.request_path)
            .field("byte_length", &self.byte_length)
            .field("digest", &"[redacted]")
            .finish()
    }
}

fn parse_format_version(fields: &BTreeMap<String, JsonValue>) -> Result<(), UpdateCatalogueError> {
    exact_fields(fields, &["major", "minor"])?;
    match (
        required_u16(fields, "major")?,
        required_u16(fields, "minor")?,
    ) {
        (1, 0) => Ok(()),
        _ => Err(UpdateCatalogueError::VersionUnsupported),
    }
}

fn parse_package_version(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<PackageVersion, UpdateCatalogueError> {
    exact_fields(fields, &["major", "minor", "patch"])?;
    Ok(PackageVersion::new(
        required_u16(fields, "major")?,
        required_u16(fields, "minor")?,
        required_u16(fields, "patch")?,
    ))
}

fn parse_publisher(fields: &BTreeMap<String, JsonValue>) -> Result<[u8; 32], UpdateCatalogueError> {
    exact_fields(fields, &["leafCertificateSha256"])?;
    sha256::parse_lower_hex(required_string(fields, "leafCertificateSha256")?)
        .ok_or(UpdateCatalogueError::Invalid)
}

fn parse_origin(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<NetworkOrigin, UpdateCatalogueError> {
    exact_fields(fields, &["host", "port"])?;
    NetworkOrigin::new(
        required_string(fields, "host")?,
        required_u16(fields, "port")?,
    )
    .map_err(|_| UpdateCatalogueError::InstallerLocationInvalid)
}

fn is_valid_request_path(value: &str) -> bool {
    (1..=512).contains(&value.len())
        && value.starts_with('/')
        && value.ends_with(".exe")
        && !value.contains("//")
        && value
            .split('/')
            .skip(1)
            .all(|component| !component.is_empty() && component != "." && component != "..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~')
        })
}

fn exact_fields(
    fields: &BTreeMap<String, JsonValue>,
    expected: &[&str],
) -> Result<(), UpdateCatalogueError> {
    (fields.len() == expected.len() && expected.iter().all(|name| fields.contains_key(*name)))
        .then_some(())
        .ok_or(UpdateCatalogueError::Invalid)
}

fn required_object<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, UpdateCatalogueError> {
    fields
        .get(name)
        .and_then(JsonValue::as_object)
        .ok_or(UpdateCatalogueError::Invalid)
}

fn required_string<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, UpdateCatalogueError> {
    fields
        .get(name)
        .and_then(JsonValue::as_string)
        .ok_or(UpdateCatalogueError::Invalid)
}

fn required_u16(
    fields: &BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<u16, UpdateCatalogueError> {
    fields
        .get(name)
        .and_then(JsonValue::as_u16)
        .ok_or(UpdateCatalogueError::Invalid)
}

fn required_u64(
    fields: &BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<u64, UpdateCatalogueError> {
    let Some(JsonValue::Number(value)) = fields.get(name) else {
        return Err(UpdateCatalogueError::InstallerBytesInvalid);
    };
    if value.starts_with('-') || value.contains(['.', 'e', 'E']) {
        return Err(UpdateCatalogueError::InstallerBytesInvalid);
    }
    value
        .parse()
        .map_err(|_| UpdateCatalogueError::InstallerBytesInvalid)
}
