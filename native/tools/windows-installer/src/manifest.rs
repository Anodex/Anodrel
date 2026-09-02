//! Strict `anodrel.release.v1` manifest parsing.

use std::collections::BTreeMap;

use anodrel_application::{is_valid_application_id, sha256};
use anodrel_json::JsonValue;
use anodrel_network::{NetworkOrigin, NetworkOriginPolicy};
use anodrel_protocol::Capability;

use crate::{MAX_PAYLOAD_BYTES, MAX_RELEASE_MANIFEST_BYTES, ReleaseManifestError};

/// One release directory version, distinct from protocol compatibility.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackageVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

impl PackageVersion {
    /// Creates one exact three-component release version.
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns the release version's major component.
    #[must_use]
    pub const fn major(self) -> u16 {
        self.major
    }

    /// Returns the release version's minor component.
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }

    /// Returns the release version's patch component.
    #[must_use]
    pub const fn patch(self) -> u16 {
        self.patch
    }

    /// Parses the exact canonical name of an owned release directory.
    ///
    /// Release promotion creates only `major.minor.patch` names with ordinary
    /// decimal components. Rejecting alternate spellings keeps a version
    /// directory from having more than one textual identity.
    pub fn from_canonical_directory_name(name: &str) -> Option<Self> {
        let mut components = name.split('.');
        let major = parse_directory_component(components.next()?)?;
        let minor = parse_directory_component(components.next()?)?;
        let patch = parse_directory_component(components.next()?)?;
        components.next().is_none().then_some(Self {
            major,
            minor,
            patch,
        })
    }
}

fn parse_directory_component(component: &str) -> Option<u16> {
    (!component.is_empty()
        && component.bytes().all(|byte| byte.is_ascii_digit())
        && (component.len() == 1 || !component.starts_with('0')))
    .then(|| component.parse().ok())?
}

/// The bounded embedded-payload facts a signed manifest declares.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PayloadDescriptor {
    byte_length: u64,
    digest: [u8; 32],
}

impl PayloadDescriptor {
    /// Returns the exact uncompressed payload byte length.
    #[must_use]
    pub const fn byte_length(self) -> u64 {
        self.byte_length
    }

    /// Compares a calculated payload digest without exposing it as display text.
    #[must_use]
    pub fn matches_digest(self, actual: [u8; 32]) -> bool {
        self.digest == actual
    }
}

impl std::fmt::Debug for PayloadDescriptor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PayloadDescriptor")
            .field("byte_length", &self.byte_length)
            .field("digest", &"[redacted]")
            .finish()
    }
}

/// One exact application release selected by a signed installer image.
pub struct ReleaseManifest {
    application_id: String,
    package_version: PackageVersion,
    executable_path: String,
    executable_digest: [u8; 32],
    publisher_fingerprint: [u8; 32],
    capabilities: Vec<Capability>,
    network_origins: Vec<NetworkOrigin>,
    payload: PayloadDescriptor,
}

impl ReleaseManifest {
    /// Parses one strict version-1 `anodrel.release.v1` manifest.
    pub fn parse(input: &str) -> Result<Self, ReleaseManifestError> {
        if input.len() > MAX_RELEASE_MANIFEST_BYTES {
            return Err(ReleaseManifestError::TooLarge);
        }
        let root = JsonValue::parse(input).map_err(|_| ReleaseManifestError::Invalid)?;
        let fields = root.as_object().ok_or(ReleaseManifestError::Invalid)?;
        exact_fields(
            fields,
            &[
                "formatVersion",
                "applicationId",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "payload",
            ],
        )?;
        parse_format_version(required_object(fields, "formatVersion")?)?;

        let application_id = required_string(fields, "applicationId")?;
        if !is_valid_application_id(application_id) {
            return Err(ReleaseManifestError::Invalid);
        }

        let package_version = parse_package_version(required_object(fields, "packageVersion")?)?;
        let (executable_path, executable_digest) =
            parse_executable(required_object(fields, "executable")?)?;
        let publisher_fingerprint = parse_publisher(required_object(fields, "publisher")?)?;
        let capabilities = parse_capabilities(fields.get("capabilities"))?;
        let network_origins = parse_network_origins(fields.get("networkOrigins"), &capabilities)?;
        let payload = parse_payload(required_object(fields, "payload")?)?;

        Ok(Self {
            application_id: application_id.to_owned(),
            package_version,
            executable_path,
            executable_digest,
            publisher_fingerprint,
            capabilities,
            network_origins,
            payload,
        })
    }

    /// Returns the signed application identity selected for this release.
    #[must_use]
    pub fn application_id(&self) -> &str {
        &self.application_id
    }

    /// Returns the signed version used for the owned release directory.
    #[must_use]
    pub const fn package_version(&self) -> PackageVersion {
        self.package_version
    }

    /// Returns the relative contained executable path.
    #[must_use]
    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }

    /// Compares a calculated executable digest without exposing it as text.
    #[must_use]
    pub fn matches_executable_digest(&self, actual: [u8; 32]) -> bool {
        self.executable_digest == actual
    }

    pub(super) const fn executable_digest(&self) -> &[u8; 32] {
        &self.executable_digest
    }

    /// Compares a Windows Authenticode leaf fingerprint without exposing it.
    #[must_use]
    pub fn matches_publisher_fingerprint(&self, actual: [u8; 32]) -> bool {
        self.publisher_fingerprint == actual
    }

    pub(super) const fn publisher_fingerprint(&self) -> &[u8; 32] {
        &self.publisher_fingerprint
    }

    /// Returns the machine-selected capability set embedded in this release.
    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Returns the exact embedded network origins.
    #[must_use]
    pub fn network_origins(&self) -> &[NetworkOrigin] {
        &self.network_origins
    }

    /// Returns the signed embedded-payload descriptor.
    #[must_use]
    pub const fn payload(&self) -> PayloadDescriptor {
        self.payload
    }
}

impl std::fmt::Debug for ReleaseManifest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReleaseManifest")
            .field("application_id", &self.application_id)
            .field("package_version", &self.package_version)
            .field("executable_path", &self.executable_path)
            .field("executable_digest", &"[redacted]")
            .field("publisher_fingerprint", &"[redacted]")
            .field("capabilities", &self.capabilities)
            .field("network_origins", &self.network_origins)
            .field("payload", &self.payload)
            .finish()
    }
}

fn parse_format_version(fields: &BTreeMap<String, JsonValue>) -> Result<(), ReleaseManifestError> {
    exact_fields(fields, &["major", "minor"])?;
    match (
        required_u16(fields, "major")?,
        required_u16(fields, "minor")?,
    ) {
        (1, 0) => Ok(()),
        (1, _) => Err(ReleaseManifestError::VersionUnsupported),
        _ => Err(ReleaseManifestError::VersionUnsupported),
    }
}

fn parse_package_version(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<PackageVersion, ReleaseManifestError> {
    exact_fields(fields, &["major", "minor", "patch"])?;
    Ok(PackageVersion {
        major: required_u16(fields, "major")?,
        minor: required_u16(fields, "minor")?,
        patch: required_u16(fields, "patch")?,
    })
}

fn parse_executable(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<(String, [u8; 32]), ReleaseManifestError> {
    exact_fields(fields, &["path", "sha256"])?;
    let path = required_string(fields, "path")?;
    if !is_valid_executable_path(path) {
        return Err(ReleaseManifestError::ExecutablePathInvalid);
    }
    let digest = sha256::parse_lower_hex(required_string(fields, "sha256")?)
        .ok_or(ReleaseManifestError::Invalid)?;
    Ok((path.to_owned(), digest))
}

fn parse_publisher(fields: &BTreeMap<String, JsonValue>) -> Result<[u8; 32], ReleaseManifestError> {
    exact_fields(fields, &["leafCertificateSha256"])?;
    sha256::parse_lower_hex(required_string(fields, "leafCertificateSha256")?)
        .ok_or(ReleaseManifestError::Invalid)
}

fn parse_capabilities(value: Option<&JsonValue>) -> Result<Vec<Capability>, ReleaseManifestError> {
    let Some(JsonValue::Array(values)) = value else {
        return Err(ReleaseManifestError::PolicyInvalid);
    };
    let mut capabilities = Vec::with_capacity(values.len());
    for value in values {
        let capability =
            capability_from_name(value.as_string()).ok_or(ReleaseManifestError::PolicyInvalid)?;
        if capabilities.contains(&capability) {
            return Err(ReleaseManifestError::PolicyInvalid);
        }
        capabilities.push(capability);
    }
    Ok(capabilities)
}

fn parse_network_origins(
    value: Option<&JsonValue>,
    capabilities: &[Capability],
) -> Result<Vec<NetworkOrigin>, ReleaseManifestError> {
    let Some(JsonValue::Array(values)) = value else {
        return Err(ReleaseManifestError::PolicyInvalid);
    };
    let mut origins = Vec::with_capacity(values.len());
    for value in values {
        let fields = value
            .as_object()
            .ok_or(ReleaseManifestError::PolicyInvalid)?;
        exact_fields(fields, &["host", "port"])?;
        let origin = NetworkOrigin::new(
            required_string(fields, "host")?,
            required_u16(fields, "port")?,
        )
        .map_err(|_| ReleaseManifestError::PolicyInvalid)?;
        origins.push(origin);
    }

    if capabilities.contains(&Capability::NetworkFetch) {
        NetworkOriginPolicy::new(origins.clone())
            .map_err(|_| ReleaseManifestError::PolicyInvalid)?;
    } else if !origins.is_empty() {
        return Err(ReleaseManifestError::PolicyInvalid);
    }
    Ok(origins)
}

fn parse_payload(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<PayloadDescriptor, ReleaseManifestError> {
    exact_fields(fields, &["byteLength", "sha256"])?;
    let byte_length = required_u64(fields, "byteLength")?;
    if !(1..=MAX_PAYLOAD_BYTES).contains(&byte_length) {
        return Err(ReleaseManifestError::PayloadInvalid);
    }
    let digest = sha256::parse_lower_hex(required_string(fields, "sha256")?)
        .ok_or(ReleaseManifestError::PayloadInvalid)?;
    Ok(PayloadDescriptor {
        byte_length,
        digest,
    })
}

fn capability_from_name(value: Option<&str>) -> Option<Capability> {
    match value? {
        "diagnostics.read" => Some(Capability::DiagnosticsRead),
        "ui.document.write" => Some(Capability::UiDocumentWrite),
        "ui.events.read" => Some(Capability::UiEventsRead),
        "session.close" => Some(Capability::SessionClose),
        "clipboard.read" => Some(Capability::ClipboardRead),
        "clipboard.write" => Some(Capability::ClipboardWrite),
        "external.open" => Some(Capability::ExternalOpen),
        "network.fetch" => Some(Capability::NetworkFetch),
        "dialog.open_file" => Some(Capability::DialogOpenFile),
        "dialog.open_folder" => Some(Capability::DialogOpenFolder),
        "folder.read_entries" => Some(Capability::FolderReadEntries),
        "dialog.save_file" => Some(Capability::DialogSaveFile),
        "file.read_text" => Some(Capability::FileReadText),
        "file.write_text" => Some(Capability::FileWriteText),
        "file.write_binary" => Some(Capability::FileWriteBinary),
        "storage.state.read" => Some(Capability::StorageStateRead),
        "storage.state.replace" => Some(Capability::StorageStateReplace),
        "storage.state.clear" => Some(Capability::StorageStateClear),
        "credential.read" => Some(Capability::CredentialRead),
        "credential.write" => Some(Capability::CredentialWrite),
        "credential.delete" => Some(Capability::CredentialDelete),
        "notification.show" => Some(Capability::NotificationShow),
        "window.title" => Some(Capability::WindowTitle),
        "window.state" => Some(Capability::WindowState),
        "window.state.read" => Some(Capability::WindowStateRead),
        "window.state.observe" => Some(Capability::WindowStateObserve),
        "window.focus" => Some(Capability::WindowFocus),
        "window.fullscreen" => Some(Capability::WindowFullscreen),
        "window.size" => Some(Capability::WindowSize),
        "window.open" => Some(Capability::WindowOpen),
        "window.close" => Some(Capability::WindowClose),
        "ui.fields.read" => Some(Capability::UiFieldsRead),
        "menu.write" => Some(Capability::MenuWrite),
        "menu.context.write" => Some(Capability::ContextMenuWrite),
        _ => None,
    }
}

fn exact_fields(
    fields: &BTreeMap<String, JsonValue>,
    expected: &[&str],
) -> Result<(), ReleaseManifestError> {
    (fields.len() == expected.len() && expected.iter().all(|name| fields.contains_key(*name)))
        .then_some(())
        .ok_or(ReleaseManifestError::Invalid)
}

fn required_object<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, ReleaseManifestError> {
    fields
        .get(name)
        .and_then(JsonValue::as_object)
        .ok_or(ReleaseManifestError::Invalid)
}

fn required_string<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, ReleaseManifestError> {
    fields
        .get(name)
        .and_then(JsonValue::as_string)
        .ok_or(ReleaseManifestError::Invalid)
}

fn required_u16(
    fields: &BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<u16, ReleaseManifestError> {
    fields
        .get(name)
        .and_then(JsonValue::as_u16)
        .ok_or(ReleaseManifestError::Invalid)
}

fn required_u64(
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

fn is_valid_executable_path(path: &str) -> bool {
    !path.is_empty()
        && path.to_ascii_lowercase().ends_with(".exe")
        && !path.contains(['\\', ':'])
        && path.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.chars().any(char::is_control)
        })
}
