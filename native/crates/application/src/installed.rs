//! Strict installed application-record validation.
//!
//! The record is selected by a host-controlled policy store outside the
//! application package. This module validates that record's package and
//! executable binding but does not select policy storage, verify Authenticode,
//! launch a process, or expose policy data to an application surface.

use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use anodrel_json::JsonValue;
use anodrel_network::NetworkOriginPolicy;
use anodrel_protocol::Capability;

use crate::{
    ApplicationError, ApplicationIdentity, ApplicationPackage, MAX_EXECUTABLE_BYTES,
    MAX_INSTALL_RECORD_BYTES, manifest, network_policy, sha256,
};

const PACKAGE_MANIFEST_NAME: &str = "anodrel.application.json";

/// A fixed SHA-256 fingerprint for the publisher approved by an installed
/// application record.
///
/// The value has no display implementation so it cannot accidentally become a
/// certificate diagnostic surface.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct PublisherFingerprint([u8; 32]);

impl fmt::Debug for PublisherFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PublisherFingerprint(..)")
    }
}

/// A record and local files that have passed the installed-record foundation
/// checks.
///
/// This does not authorize a process launch. A Windows launch service must
/// revalidate the executable, compare an Authenticode result, and manage the
/// child lifecycle immediately before process creation.
pub struct InstalledApplication {
    identity: ApplicationIdentity,
    package_root: PathBuf,
    executable_path: PathBuf,
    executable_digest: [u8; 32],
    publisher_fingerprint: PublisherFingerprint,
    capabilities: Vec<Capability>,
    network_policy: Option<NetworkOriginPolicy>,
}

impl InstalledApplication {
    /// Loads a record selected from a host-controlled policy root and verifies
    /// the package and executable facts it names.
    pub fn load(
        record_path: impl AsRef<Path>,
        policy_root: impl AsRef<Path>,
    ) -> Result<Self, InstalledApplicationError> {
        let policy_root = canonical_directory(policy_root.as_ref())
            .map_err(|_| InstalledApplicationError::InvalidPolicyRoot)?;
        let record_path = fs::canonicalize(record_path).map_err(InstalledApplicationError::Io)?;
        if !record_path.starts_with(&policy_root) {
            return Err(InstalledApplicationError::RecordOutsidePolicyRoot);
        }
        if !fs::metadata(&record_path)
            .map_err(InstalledApplicationError::Io)?
            .is_file()
        {
            return Err(InstalledApplicationError::InvalidRecord);
        }

        let record = parse_record(&read_limited(&record_path, MAX_INSTALL_RECORD_BYTES)?)?;
        validate_record(record, Some(&record_path), None)
    }

    /// Validates a record supplied by a native operating-system policy source.
    ///
    /// The caller must select the source independently of every application,
    /// package, protocol, environment, and UI value. The expected identity is
    /// supplied by the host-selected policy key and must match both the record
    /// and its package manifest.
    pub fn load_from_trusted_record(
        record: &str,
        expected_application_id: &str,
    ) -> Result<Self, InstalledApplicationError> {
        if !manifest::is_valid_application_id(expected_application_id) {
            return Err(InstalledApplicationError::InvalidRecord);
        }
        let record = parse_record(record)?;
        validate_record(record, None, Some(expected_application_id))
    }

    /// Returns the package identity that exactly matched the installed record.
    #[must_use]
    pub fn identity(&self) -> &ApplicationIdentity {
        &self.identity
    }

    /// Returns the canonical executable path for a native launch service.
    ///
    /// This is private host data and must never be sent to an application or
    /// renderer.
    #[must_use]
    pub fn executable_path(&self) -> &Path {
        &self.executable_path
    }

    /// Compares a newly calculated executable digest with the record's expected
    /// digest without exposing the expected value.
    #[must_use]
    pub fn matches_executable_digest(&self, actual: [u8; 32]) -> bool {
        self.executable_digest == actual
    }

    /// Compares an Authenticode leaf fingerprint with the approved publisher
    /// without exposing the approved value.
    #[must_use]
    pub fn matches_publisher(&self, actual: [u8; 32]) -> bool {
        self.publisher_fingerprint.0 == actual
    }

    /// Returns the machine-policy grants for a future authenticated child session.
    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Returns the exact machine-selected HTTPS policy, when this record
    /// deliberately grants bounded text fetches.
    ///
    /// This is private host-composition data. It is never serialized into a
    /// protocol response or renderer value, and an application cannot inspect
    /// or change it.
    #[must_use]
    pub fn network_origin_policy(&self) -> Option<&NetworkOriginPolicy> {
        self.network_policy.as_ref()
    }

    /// Rechecks an executable path and hashes bytes read from a caller-held
    /// file handle against this record's expected digest.
    ///
    /// A native launch service must call this while it holds operating-system
    /// protection against replacing the executable, then keep that protection
    /// until process creation has returned.
    pub fn revalidate_executable<R: Read>(
        &self,
        path: &Path,
        reader: &mut R,
    ) -> Result<(), InstalledApplicationError> {
        let canonical_path = fs::canonicalize(path).map_err(InstalledApplicationError::Io)?;
        if !canonical_path.starts_with(&self.package_root) {
            return Err(InstalledApplicationError::ExecutableOutsidePackage);
        }
        if canonical_path != self.executable_path {
            return Err(InstalledApplicationError::ExecutablePathChanged);
        }
        let (actual_digest, _) = sha256::digest_reader_limited(reader, MAX_EXECUTABLE_BYTES)
            .map_err(InstalledApplicationError::Io)?
            .ok_or(InstalledApplicationError::ExecutableTooLarge)?;
        if self.matches_executable_digest(actual_digest) {
            Ok(())
        } else {
            Err(InstalledApplicationError::ExecutableDigestMismatch)
        }
    }
}

impl fmt::Debug for InstalledApplication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InstalledApplication(..)")
    }
}

fn validate_record(
    record: ParsedRecord,
    record_path: Option<&Path>,
    expected_application_id: Option<&str>,
) -> Result<InstalledApplication, InstalledApplicationError> {
    if expected_application_id.is_some_and(|expected| expected != record.application_id) {
        return Err(InstalledApplicationError::ApplicationIdentityMismatch);
    }

    let package_root = canonical_directory(Path::new(&record.package_root))
        .map_err(|_| InstalledApplicationError::InvalidPackageRoot)?;
    if record_path.is_some_and(|path| path.starts_with(&package_root)) {
        return Err(InstalledApplicationError::RecordInsidePackage);
    }

    let manifest_path = fs::canonicalize(package_root.join(PACKAGE_MANIFEST_NAME))
        .map_err(InstalledApplicationError::Io)?;
    if !manifest_path.starts_with(&package_root) {
        return Err(InstalledApplicationError::PackageManifestOutsidePackage);
    }
    let package =
        ApplicationPackage::load(&manifest_path).map_err(InstalledApplicationError::Package)?;
    if package.identity().application_id() != record.application_id {
        return Err(InstalledApplicationError::ApplicationIdentityMismatch);
    }

    let executable_path = canonical_executable_path(&package_root, &record.executable_path)?;
    let (actual_digest, _) = digest_file(&executable_path)?;
    if actual_digest != record.executable_digest {
        return Err(InstalledApplicationError::ExecutableDigestMismatch);
    }

    Ok(InstalledApplication {
        identity: package.identity().clone(),
        package_root,
        executable_path,
        executable_digest: record.executable_digest,
        publisher_fingerprint: PublisherFingerprint(record.publisher_fingerprint),
        capabilities: record.capabilities,
        network_policy: record.network_policy,
    })
}

/// A safe failure category while loading an installed application record.
#[derive(Debug)]
pub enum InstalledApplicationError {
    Io(std::io::Error),
    InvalidPolicyRoot,
    RecordTooLarge,
    InvalidRecord,
    RecordOutsidePolicyRoot,
    InvalidPackageRoot,
    RecordInsidePackage,
    PackageManifestOutsidePackage,
    Package(ApplicationError),
    ApplicationIdentityMismatch,
    InvalidExecutablePath,
    ExecutableOutsidePackage,
    ExecutablePathChanged,
    ExecutableNotFile,
    ExecutableTooLarge,
    ExecutableDigestMismatch,
}

impl fmt::Display for InstalledApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io(_) => "installed application record could not be read",
            Self::InvalidPolicyRoot => "installed application policy root is invalid",
            Self::RecordTooLarge => "installed application record exceeds its limit",
            Self::InvalidRecord => "installed application record is invalid",
            Self::RecordOutsidePolicyRoot => "installed application record is outside policy root",
            Self::InvalidPackageRoot => "installed application package root is invalid",
            Self::RecordInsidePackage => "installed application record is inside its package",
            Self::PackageManifestOutsidePackage => {
                "installed application manifest resolves outside its package"
            }
            Self::Package(_) => "installed application package validation failed",
            Self::ApplicationIdentityMismatch => {
                "installed application identity does not match its package"
            }
            Self::InvalidExecutablePath => "installed application executable path is invalid",
            Self::ExecutableOutsidePackage => {
                "installed application executable resolves outside its package"
            }
            Self::ExecutablePathChanged => {
                "installed application executable changed after policy validation"
            }
            Self::ExecutableNotFile => "installed application executable is not a file",
            Self::ExecutableTooLarge => "installed application executable exceeds its limit",
            Self::ExecutableDigestMismatch => {
                "installed application executable digest does not match"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for InstalledApplicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Package(error) => Some(error),
            _ => None,
        }
    }
}

struct ParsedRecord {
    application_id: String,
    package_root: String,
    executable_path: String,
    executable_digest: [u8; 32],
    publisher_fingerprint: [u8; 32],
    capabilities: Vec<Capability>,
    network_policy: Option<NetworkOriginPolicy>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RecordVersion {
    V1_0,
    V1_1,
    V1_2,
    V1_3,
    V1_4,
    V1_5,
    V1_6,
    V1_7,
    V1_8,
    V1_9,
    V1_10,
    V1_11,
    V1_12,
    V1_13,
    V1_14,
}

impl RecordVersion {
    /// Returns whether this strictly ordered record version includes grants
    /// introduced by `minimum`. Keeping the comparison here prevents a newer
    /// record from accidentally losing an older optional grant when another
    /// version is added.
    const fn accepts(self, minimum: Self) -> bool {
        (self as u8) >= (minimum as u8)
    }

    /// Whether this version accepts the grants version 1.2 introduced.
    ///
    /// Each later version is a superset, so a record written for 1.2 keeps its
    /// exact meaning when read as 1.3 or 1.4.
    const fn accepts_v1_2_grants(self) -> bool {
        self.accepts(Self::V1_2)
    }

    /// Whether this version accepts the grant version 1.3 introduced.
    const fn accepts_v1_3_grants(self) -> bool {
        self.accepts(Self::V1_3)
    }

    /// Whether this version accepts the grant version 1.4 introduced.
    const fn accepts_v1_4_grants(self) -> bool {
        self.accepts(Self::V1_4)
    }

    /// Whether this version accepts the grant version 1.5 introduced.
    const fn accepts_v1_5_grants(self) -> bool {
        self.accepts(Self::V1_5)
    }
}

fn parse_record(input: &str) -> Result<ParsedRecord, InstalledApplicationError> {
    if input.len() > MAX_INSTALL_RECORD_BYTES {
        return Err(InstalledApplicationError::RecordTooLarge);
    }
    let root = JsonValue::parse(input).map_err(|_| InstalledApplicationError::InvalidRecord)?;
    let fields = root
        .as_object()
        .ok_or(InstalledApplicationError::InvalidRecord)?;
    let version = validate_version(required_object(fields, "recordVersion")?)?;
    let expected_fields = if version == RecordVersion::V1_0 {
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
        ][..]
    } else if version == RecordVersion::V1_14 {
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
            "capabilities",
            "networkOrigins",
        ][..]
    } else {
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
            "capabilities",
        ][..]
    };
    exact_fields(fields, expected_fields)?;

    let application_id = required_string(fields, "applicationId")?;
    if !manifest::is_valid_application_id(application_id) {
        return Err(InstalledApplicationError::InvalidRecord);
    }
    let package_root = required_string(fields, "packageRoot")?;
    if package_root.is_empty() || package_root.chars().any(char::is_control) {
        return Err(InstalledApplicationError::InvalidRecord);
    }

    let executable = required_object(fields, "executable")?;
    exact_fields(executable, &["path", "sha256"])?;
    let executable_path = required_string(executable, "path")?;
    if !is_valid_executable_path(executable_path) {
        return Err(InstalledApplicationError::InvalidExecutablePath);
    }
    let executable_digest = sha256::parse_lower_hex(required_string(executable, "sha256")?)
        .ok_or(InstalledApplicationError::InvalidRecord)?;

    let publisher = required_object(fields, "publisher")?;
    exact_fields(publisher, &["leafCertificateSha256"])?;
    let publisher_fingerprint =
        sha256::parse_lower_hex(required_string(publisher, "leafCertificateSha256")?)
            .ok_or(InstalledApplicationError::InvalidRecord)?;

    let capabilities = if version == RecordVersion::V1_0 {
        Vec::new()
    } else {
        let Some(JsonValue::Array(values)) = fields.get("capabilities") else {
            return Err(InstalledApplicationError::InvalidRecord);
        };
        let mut grants = Vec::with_capacity(values.len());
        for value in values {
            let capability = capability_for_record_version(version, value.as_string())
                .ok_or(InstalledApplicationError::InvalidRecord)?;
            if grants.contains(&capability) {
                return Err(InstalledApplicationError::InvalidRecord);
            }
            grants.push(capability);
        }
        grants
    };
    let network_policy = if version == RecordVersion::V1_14 {
        network_policy::parse_network_policy(
            fields,
            capabilities.contains(&Capability::NetworkFetch),
        )?
    } else {
        None
    };

    Ok(ParsedRecord {
        application_id: application_id.to_owned(),
        package_root: package_root.to_owned(),
        executable_path: executable_path.to_owned(),
        executable_digest,
        publisher_fingerprint,
        capabilities,
        network_policy,
    })
}

fn capability_for_record_version(
    version: RecordVersion,
    value: Option<&str>,
) -> Option<Capability> {
    match value? {
        "diagnostics.read" => Some(Capability::DiagnosticsRead),
        "ui.document.write" => Some(Capability::UiDocumentWrite),
        "ui.events.read" => Some(Capability::UiEventsRead),
        "session.close" => Some(Capability::SessionClose),
        "clipboard.read" => Some(Capability::ClipboardRead),
        "clipboard.write" => Some(Capability::ClipboardWrite),
        "external.open" => Some(Capability::ExternalOpen),
        "dialog.open_file" if version.accepts_v1_2_grants() => Some(Capability::DialogOpenFile),
        "dialog.save_file" if version.accepts_v1_2_grants() => Some(Capability::DialogSaveFile),
        "file.read_text" if version.accepts_v1_2_grants() => Some(Capability::FileReadText),
        "storage.state.read" if version.accepts_v1_2_grants() => Some(Capability::StorageStateRead),
        "storage.state.replace" if version.accepts_v1_2_grants() => {
            Some(Capability::StorageStateReplace)
        }
        "storage.state.clear" if version.accepts_v1_2_grants() => {
            Some(Capability::StorageStateClear)
        }
        "credential.read" if version.accepts_v1_2_grants() => Some(Capability::CredentialRead),
        "credential.write" if version.accepts_v1_2_grants() => Some(Capability::CredentialWrite),
        "credential.delete" if version.accepts_v1_2_grants() => Some(Capability::CredentialDelete),
        // Each later record version adds named grants deliberately. An earlier
        // record naming a later grant stays invalid, so provisioning cannot
        // widen a record by accident.
        "notification.show" if version.accepts_v1_3_grants() => Some(Capability::NotificationShow),
        "window.title" if version.accepts_v1_4_grants() => Some(Capability::WindowTitle),
        "ui.fields.read" if version.accepts_v1_5_grants() => Some(Capability::UiFieldsRead),
        "window.state" if version.accepts(RecordVersion::V1_6) => Some(Capability::WindowState),
        "file.write_text" if version.accepts(RecordVersion::V1_7) => {
            Some(Capability::FileWriteText)
        }
        "menu.write" if version.accepts(RecordVersion::V1_8) => Some(Capability::MenuWrite),
        "window.focus" if version.accepts(RecordVersion::V1_9) => Some(Capability::WindowFocus),
        "window.fullscreen" if version.accepts(RecordVersion::V1_10) => {
            Some(Capability::WindowFullscreen)
        }
        "file.write_binary" if version.accepts(RecordVersion::V1_11) => {
            Some(Capability::FileWriteBinary)
        }
        "window.size" if version.accepts(RecordVersion::V1_12) => Some(Capability::WindowSize),
        "window.open" if version.accepts(RecordVersion::V1_13) => Some(Capability::WindowOpen),
        "window.close" if version.accepts(RecordVersion::V1_13) => Some(Capability::WindowClose),
        "network.fetch" if version == RecordVersion::V1_14 => Some(Capability::NetworkFetch),
        _ => None,
    }
}

fn canonical_directory(path: &Path) -> std::io::Result<PathBuf> {
    let path = fs::canonicalize(path)?;
    if fs::metadata(&path)?.is_dir() {
        Ok(path)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path is not a directory",
        ))
    }
}

fn canonical_executable_path(
    package_root: &Path,
    declared_path: &str,
) -> Result<PathBuf, InstalledApplicationError> {
    let path = fs::canonicalize(package_root.join(declared_path))
        .map_err(InstalledApplicationError::Io)?;
    if !path.starts_with(package_root) {
        return Err(InstalledApplicationError::ExecutableOutsidePackage);
    }
    if !fs::metadata(&path)
        .map_err(InstalledApplicationError::Io)?
        .is_file()
    {
        return Err(InstalledApplicationError::ExecutableNotFile);
    }
    Ok(path)
}

fn digest_file(path: &Path) -> Result<([u8; 32], usize), InstalledApplicationError> {
    let mut file = File::open(path).map_err(InstalledApplicationError::Io)?;
    sha256::digest_reader_limited(&mut file, MAX_EXECUTABLE_BYTES)
        .map_err(InstalledApplicationError::Io)?
        .ok_or(InstalledApplicationError::ExecutableTooLarge)
}

fn read_limited(path: &Path, maximum: usize) -> Result<String, InstalledApplicationError> {
    let file = File::open(path).map_err(InstalledApplicationError::Io)?;
    let mut reader = file.take((maximum + 1) as u64);
    let mut contents = Vec::with_capacity(maximum.min(4_096));
    reader
        .read_to_end(&mut contents)
        .map_err(InstalledApplicationError::Io)?;
    if contents.len() > maximum {
        Err(InstalledApplicationError::RecordTooLarge)
    } else {
        String::from_utf8(contents).map_err(|_| InstalledApplicationError::InvalidRecord)
    }
}

pub(super) fn exact_fields(
    fields: &BTreeMap<String, JsonValue>,
    expected: &[&str],
) -> Result<(), InstalledApplicationError> {
    if fields.len() != expected.len() || expected.iter().any(|field| !fields.contains_key(*field)) {
        Err(InstalledApplicationError::InvalidRecord)
    } else {
        Ok(())
    }
}

pub(super) fn required_string<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<&'a str, InstalledApplicationError> {
    fields
        .get(field)
        .and_then(JsonValue::as_string)
        .ok_or(InstalledApplicationError::InvalidRecord)
}

fn required_object<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, InstalledApplicationError> {
    fields
        .get(field)
        .and_then(JsonValue::as_object)
        .ok_or(InstalledApplicationError::InvalidRecord)
}

fn validate_version(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<RecordVersion, InstalledApplicationError> {
    exact_fields(fields, &["major", "minor"])?;
    let major = fields
        .get("major")
        .and_then(JsonValue::as_u16)
        .ok_or(InstalledApplicationError::InvalidRecord)?;
    let minor = fields
        .get("minor")
        .and_then(JsonValue::as_u16)
        .ok_or(InstalledApplicationError::InvalidRecord)?;
    match (major, minor) {
        (1, 0) => Ok(RecordVersion::V1_0),
        (1, 1) => Ok(RecordVersion::V1_1),
        (1, 2) => Ok(RecordVersion::V1_2),
        (1, 3) => Ok(RecordVersion::V1_3),
        (1, 4) => Ok(RecordVersion::V1_4),
        (1, 5) => Ok(RecordVersion::V1_5),
        (1, 6) => Ok(RecordVersion::V1_6),
        (1, 7) => Ok(RecordVersion::V1_7),
        (1, 8) => Ok(RecordVersion::V1_8),
        (1, 9) => Ok(RecordVersion::V1_9),
        (1, 10) => Ok(RecordVersion::V1_10),
        (1, 11) => Ok(RecordVersion::V1_11),
        (1, 12) => Ok(RecordVersion::V1_12),
        (1, 13) => Ok(RecordVersion::V1_13),
        (1, 14) => Ok(RecordVersion::V1_14),
        _ => Err(InstalledApplicationError::InvalidRecord),
    }
}

fn is_valid_executable_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    let extension = bytes
        .get(bytes.len().saturating_sub(4)..)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(b".exe"));
    extension
        && !value.is_empty()
        && value.len() <= 240
        && !value.contains(['\\', ':'])
        && !bytes.contains(&0)
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

#[cfg(test)]
mod tests;
