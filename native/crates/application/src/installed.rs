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
use anodrel_protocol::Capability;

use crate::{
    ApplicationError, ApplicationIdentity, ApplicationPackage, MAX_EXECUTABLE_BYTES,
    MAX_INSTALL_RECORD_BYTES, manifest, sha256,
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
}

impl RecordVersion {
    /// Whether this version accepts the grants version 1.2 introduced.
    ///
    /// Each later version is a superset, so a record written for 1.2 keeps its
    /// exact meaning when read as 1.3 or 1.4.
    const fn accepts_v1_2_grants(self) -> bool {
        matches!(
            self,
            Self::V1_2 | Self::V1_3 | Self::V1_4 | Self::V1_5 | Self::V1_6
        )
    }

    /// Whether this version accepts the grant version 1.3 introduced.
    const fn accepts_v1_3_grants(self) -> bool {
        matches!(self, Self::V1_3 | Self::V1_4 | Self::V1_5 | Self::V1_6)
    }

    /// Whether this version accepts the grant version 1.4 introduced.
    const fn accepts_v1_4_grants(self) -> bool {
        matches!(self, Self::V1_4 | Self::V1_5 | Self::V1_6)
    }

    /// Whether this version accepts the grant version 1.5 introduced.
    const fn accepts_v1_5_grants(self) -> bool {
        matches!(self, Self::V1_5 | Self::V1_6)
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

    Ok(ParsedRecord {
        application_id: application_id.to_owned(),
        package_root: package_root.to_owned(),
        executable_path: executable_path.to_owned(),
        executable_digest,
        publisher_fingerprint,
        capabilities,
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
        "window.state" if version == RecordVersion::V1_6 => Some(Capability::WindowState),
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

fn exact_fields(
    fields: &BTreeMap<String, JsonValue>,
    expected: &[&str],
) -> Result<(), InstalledApplicationError> {
    if fields.len() != expected.len() || expected.iter().any(|field| !fields.contains_key(*field)) {
        Err(InstalledApplicationError::InvalidRecord)
    } else {
        Ok(())
    }
}

fn required_string<'a>(
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
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{InstalledApplication, InstalledApplicationError};
    use crate::sha256;

    const APPLICATION_ID: &str = "org.anodrel.sample";

    struct Fixture {
        root: PathBuf,
        policy_root: PathBuf,
        package_root: PathBuf,
        record_path: PathBuf,
        executable_path: PathBuf,
    }

    impl Fixture {
        fn remove(self) {
            fs::remove_dir_all(self.root).expect("fixture directory is removed");
        }
    }

    fn fixture() -> Fixture {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "anodrel-installed-application-{}-{unique}",
            std::process::id()
        ));
        let policy_root = root.join("policy");
        let package_root = root.join("package");
        let content_path = package_root.join("content").join("main.txt");
        let executable_path = package_root.join("bin").join("sample.exe");
        fs::create_dir_all(content_path.parent().expect("content has parent"))
            .expect("content directory is created");
        fs::create_dir_all(executable_path.parent().expect("executable has parent"))
            .expect("executable directory is created");
        fs::create_dir_all(&policy_root).expect("policy directory is created");

        let content = b"Hello from the installed package.\n";
        let executable = b"Anodrel fixture executable";
        fs::write(&content_path, content).expect("content is written");
        fs::write(&executable_path, executable).expect("executable is written");
        fs::write(
            package_root.join("anodrel.application.json"),
            format!(
                r#"{{
                    "manifestVersion": {{"major": 1, "minor": 0}},
                    "applicationId": "{APPLICATION_ID}",
                    "displayName": "Anodrel Sample",
                    "content": {{
                        "format": "anodrel.text.v1",
                        "path": "content/main.txt",
                        "sha256": "{}"
                    }}
                }}"#,
                sha256::to_lower_hex(&sha256::digest(content)),
            ),
        )
        .expect("package manifest is written");

        let record_path = policy_root.join("sample.json");
        write_record(
            &record_path,
            APPLICATION_ID,
            &package_root,
            "bin/sample.exe",
            &sha256::to_lower_hex(&sha256::digest(executable)),
        );

        Fixture {
            root,
            policy_root,
            package_root,
            record_path,
            executable_path,
        }
    }

    fn write_record(
        path: &Path,
        application_id: &str,
        package_root: &Path,
        executable_path: &str,
        executable_digest: &str,
    ) {
        let package_root = package_root
            .to_str()
            .expect("temporary path is valid Unicode")
            .replace('\\', "\\\\");
        fs::write(
            path,
            format!(
                r#"{{
                    "recordVersion": {{"major": 1, "minor": 0}},
                    "applicationId": "{application_id}",
                    "packageRoot": "{package_root}",
                    "executable": {{
                        "path": "{executable_path}",
                        "sha256": "{executable_digest}"
                    }},
                    "publisher": {{
                        "leafCertificateSha256": "{}"
                    }}
                }}"#,
                sha256::to_lower_hex(&[0xA5; 32]),
            ),
        )
        .expect("installed record is written");
    }

    #[test]
    fn loads_a_record_that_binds_package_executable_and_publisher() {
        let fixture = fixture();
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("installed record is valid");

        assert_eq!(installed.identity().application_id(), APPLICATION_ID);
        assert_eq!(
            installed.executable_path(),
            fs::canonicalize(&fixture.executable_path)
                .as_deref()
                .expect("executable canonicalizes")
        );
        assert!(installed.matches_executable_digest(sha256::digest(b"Anodrel fixture executable")));
        assert!(installed.matches_publisher([0xA5; 32]));
        assert!(!installed.matches_publisher([0x5A; 32]));
        assert_eq!(format!("{installed:?}"), "InstalledApplication(..)");
        fixture.remove();
    }

    #[test]
    fn loads_a_trusted_operating_system_record_with_a_matching_identity() {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path).expect("record is read");
        let installed = InstalledApplication::load_from_trusted_record(&record, APPLICATION_ID)
            .expect("trusted record is valid");

        assert_eq!(installed.identity().application_id(), APPLICATION_ID);
        fixture.remove();
    }

    #[test]
    fn revalidation_hashes_the_record_executable_and_rejects_a_substitute_path() {
        let fixture = fixture();
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("installed record is valid");
        let mut executable = fs::File::open(&fixture.executable_path).expect("executable opens");
        installed
            .revalidate_executable(&fixture.executable_path, &mut executable)
            .expect("record executable revalidates");

        let substitute = fixture.package_root.join("bin").join("substitute.exe");
        fs::write(&substitute, b"Anodrel fixture executable").expect("substitute is written");
        let mut substitute_file = fs::File::open(&substitute).expect("substitute opens");
        assert!(matches!(
            installed.revalidate_executable(&substitute, &mut substitute_file),
            Err(InstalledApplicationError::ExecutablePathChanged)
        ));
        fixture.remove();
    }

    #[test]
    fn record_v1_1_accepts_only_supported_machine_grants() {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path).expect("record is read");
        let record = record.replace("\"minor\": 0", "\"minor\": 1").replace(
            "\"publisher\": {",
            "\"capabilities\": [\"diagnostics.read\"], \"publisher\": {",
        );
        fs::write(&fixture.record_path, record).expect("record is updated");
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("v1.1 record is valid");
        assert_eq!(
            installed.capabilities(),
            &[anodrel_protocol::Capability::DiagnosticsRead]
        );

        let ui_grant = fs::read_to_string(&fixture.record_path)
            .expect("validated record is read")
            .replace("diagnostics.read", "ui.document.write");
        fs::write(&fixture.record_path, ui_grant).expect("record is updated");
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("UI document grant is valid");
        assert_eq!(
            installed.capabilities(),
            &[anodrel_protocol::Capability::UiDocumentWrite]
        );

        let event_grant = fs::read_to_string(&fixture.record_path)
            .expect("validated record is read")
            .replace("ui.document.write", "ui.events.read");
        fs::write(&fixture.record_path, event_grant).expect("record is updated");
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("UI event grant is valid");
        assert_eq!(
            installed.capabilities(),
            &[anodrel_protocol::Capability::UiEventsRead]
        );

        let close_grant = fs::read_to_string(&fixture.record_path)
            .expect("validated record is read")
            .replace("ui.events.read", "session.close");
        fs::write(&fixture.record_path, close_grant).expect("record is updated");
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("session close grant is valid");
        assert_eq!(
            installed.capabilities(),
            &[anodrel_protocol::Capability::SessionClose]
        );

        let clipboard_read_grant = fs::read_to_string(&fixture.record_path)
            .expect("validated record is read")
            .replace("session.close", "clipboard.read");
        fs::write(&fixture.record_path, clipboard_read_grant).expect("record is updated");
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("clipboard read grant is valid");
        assert_eq!(
            installed.capabilities(),
            &[anodrel_protocol::Capability::ClipboardRead]
        );

        let clipboard_write_grant = fs::read_to_string(&fixture.record_path)
            .expect("validated record is read")
            .replace("clipboard.read", "clipboard.write");
        fs::write(&fixture.record_path, clipboard_write_grant).expect("record is updated");
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("clipboard write grant is valid");
        assert_eq!(
            installed.capabilities(),
            &[anodrel_protocol::Capability::ClipboardWrite]
        );

        let external_open_grant = fs::read_to_string(&fixture.record_path)
            .expect("validated record is read")
            .replace("clipboard.write", "external.open");
        fs::write(&fixture.record_path, external_open_grant).expect("record is updated");
        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("external open grant is valid");
        assert_eq!(
            installed.capabilities(),
            &[anodrel_protocol::Capability::ExternalOpen]
        );

        let unsupported = fs::read_to_string(&fixture.record_path)
            .expect("validated record is read")
            .replace("external.open", "credential.read");
        fs::write(&fixture.record_path, unsupported).expect("record is updated");
        assert!(matches!(
            InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
            Err(InstalledApplicationError::InvalidRecord)
        ));
        fixture.remove();
    }

    #[test]
    fn record_v1_2_accepts_the_newly_composable_storage_and_credential_grants() {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", "\"minor\": 2")
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"storage.state.read\", \"storage.state.replace\", \"storage.state.clear\", \"credential.read\", \"credential.write\", \"credential.delete\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("v1.2 record is valid");
        assert_eq!(
            installed.capabilities(),
            &[
                anodrel_protocol::Capability::StorageStateRead,
                anodrel_protocol::Capability::StorageStateReplace,
                anodrel_protocol::Capability::StorageStateClear,
                anodrel_protocol::Capability::CredentialRead,
                anodrel_protocol::Capability::CredentialWrite,
                anodrel_protocol::Capability::CredentialDelete,
            ]
        );
        fixture.remove();
    }

    #[test]
    fn record_v1_3_adds_notifications_and_keeps_every_earlier_grant() {
        // Each version is a superset, so a record written for 1.2 must keep its
        // exact meaning when its version is raised.
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", "\"minor\": 3")
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"clipboard.read\", \"credential.read\", \"notification.show\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("v1.3 record is valid");
        assert_eq!(
            installed.capabilities(),
            &[
                anodrel_protocol::Capability::ClipboardRead,
                anodrel_protocol::Capability::CredentialRead,
                anodrel_protocol::Capability::NotificationShow,
            ]
        );
        fixture.remove();
    }

    #[test]
    fn record_v1_4_adds_the_window_title_grant_and_keeps_every_earlier_grant() {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", "\"minor\": 4")
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"clipboard.read\", \"credential.read\", \"notification.show\", \"window.title\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("v1.4 record is valid");
        assert_eq!(
            installed.capabilities(),
            &[
                anodrel_protocol::Capability::ClipboardRead,
                anodrel_protocol::Capability::CredentialRead,
                anodrel_protocol::Capability::NotificationShow,
                anodrel_protocol::Capability::WindowTitle,
            ]
        );
        fixture.remove();
    }

    #[test]
    fn record_v1_5_adds_the_field_read_grant_and_keeps_every_earlier_grant() {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", "\"minor\": 5")
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"clipboard.read\", \"notification.show\", \"window.title\", \"ui.fields.read\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("v1.5 record is valid");
        assert_eq!(
            installed.capabilities(),
            &[
                anodrel_protocol::Capability::ClipboardRead,
                anodrel_protocol::Capability::NotificationShow,
                anodrel_protocol::Capability::WindowTitle,
                anodrel_protocol::Capability::UiFieldsRead,
            ]
        );
        fixture.remove();
    }

    #[test]
    fn record_v1_6_adds_window_state_and_keeps_every_earlier_grant() {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", "\"minor\": 6")
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"clipboard.read\", \"window.title\", \"ui.fields.read\", \"window.state\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
            .expect("v1.6 record is valid");
        assert_eq!(
            installed.capabilities(),
            &[
                anodrel_protocol::Capability::ClipboardRead,
                anodrel_protocol::Capability::WindowTitle,
                anodrel_protocol::Capability::UiFieldsRead,
                anodrel_protocol::Capability::WindowState,
            ]
        );
        fixture.remove();
    }

    #[test]
    fn an_earlier_record_cannot_name_the_field_read_grant() {
        // 1.4 is the case that matters: the newest version predating this
        // grant, so the one a stale provisioning step would still be writing.
        for minor in ["2", "3", "4"] {
            let fixture = fixture();
            let record = fs::read_to_string(&fixture.record_path)
                .expect("record is read")
                .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
                .replace(
                    "\"publisher\": {",
                    "\"capabilities\": [\"ui.fields.read\"], \"publisher\": {",
                );
            fs::write(&fixture.record_path, record).expect("record is updated");

            assert!(
                matches!(
                    InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                    Err(InstalledApplicationError::InvalidRecord)
                ),
                "record version 1.{minor} accepted a 1.5 grant"
            );
            fixture.remove();
        }
    }

    #[test]
    fn an_earlier_record_cannot_name_the_window_state_grant() {
        // Version 1.5 is the newest record before this grant. Keeping it
        // invalid prevents a stale provisioning tool from silently widening a
        // verified application's window authority.
        for minor in ["3", "4", "5"] {
            let fixture = fixture();
            let record = fs::read_to_string(&fixture.record_path)
                .expect("record is read")
                .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
                .replace(
                    "\"publisher\": {",
                    "\"capabilities\": [\"window.state\"], \"publisher\": {",
                );
            fs::write(&fixture.record_path, record).expect("record is updated");

            assert!(
                matches!(
                    InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                    Err(InstalledApplicationError::InvalidRecord)
                ),
                "record version 1.{minor} accepted a 1.6 grant"
            );
            fixture.remove();
        }
    }

    #[test]
    fn an_earlier_record_cannot_name_the_window_title_grant() {
        // The same widening guard as the notification grant, one version later.
        // Version 1.3 is the interesting case: it is the newest version that
        // predates this grant, so it is the one a stale provisioning step would
        // most plausibly still be writing.
        for minor in ["1", "2", "3"] {
            let fixture = fixture();
            let record = fs::read_to_string(&fixture.record_path)
                .expect("record is read")
                .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
                .replace(
                    "\"publisher\": {",
                    "\"capabilities\": [\"window.title\"], \"publisher\": {",
                );
            fs::write(&fixture.record_path, record).expect("record is updated");

            assert!(
                matches!(
                    InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                    Err(InstalledApplicationError::InvalidRecord)
                ),
                "record version 1.{minor} accepted a 1.4 grant"
            );
            fixture.remove();
        }
    }

    #[test]
    fn an_earlier_record_cannot_name_the_notification_grant() {
        // Provisioning must not be able to widen a record by naming a grant its
        // declared version does not carry.
        for minor in ["1", "2"] {
            let fixture = fixture();
            let record = fs::read_to_string(&fixture.record_path)
                .expect("record is read")
                .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
                .replace(
                    "\"publisher\": {",
                    "\"capabilities\": [\"notification.show\"], \"publisher\": {",
                );
            fs::write(&fixture.record_path, record).expect("record is updated");

            assert!(matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ));
            fixture.remove();
        }
    }

    #[test]
    fn rejects_a_trusted_record_for_a_different_policy_key_identity() {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path).expect("record is read");

        assert!(matches!(
            InstalledApplication::load_from_trusted_record(&record, "org.anodrel.other"),
            Err(InstalledApplicationError::ApplicationIdentityMismatch)
        ));
        fixture.remove();
    }

    #[test]
    fn rejects_a_record_that_disagrees_with_the_package_identity() {
        let fixture = fixture();
        let contents = fs::read_to_string(&fixture.record_path).expect("record is read");
        fs::write(
            &fixture.record_path,
            contents.replacen(APPLICATION_ID, "org.anodrel.other", 1),
        )
        .expect("record is changed");

        assert!(matches!(
            InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
            Err(InstalledApplicationError::ApplicationIdentityMismatch)
        ));
        fixture.remove();
    }

    #[test]
    fn rejects_a_record_outside_the_selected_policy_root() {
        let fixture = fixture();
        let other_policy_root = fixture.root.join("other-policy");
        fs::create_dir(&other_policy_root).expect("other policy directory is created");

        assert!(matches!(
            InstalledApplication::load(&fixture.record_path, &other_policy_root),
            Err(InstalledApplicationError::RecordOutsidePolicyRoot)
        ));
        fixture.remove();
    }

    #[test]
    fn rejects_an_executable_that_changes_after_the_record_is_written() {
        let fixture = fixture();
        fs::write(&fixture.executable_path, b"changed executable").expect("executable is changed");

        assert!(matches!(
            InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
            Err(InstalledApplicationError::ExecutableDigestMismatch)
        ));
        fixture.remove();
    }

    #[test]
    fn rejects_a_record_inside_its_mutable_package() {
        let fixture = fixture();
        let inside_package = fixture.package_root.join("installed.json");
        fs::copy(&fixture.record_path, &inside_package).expect("record is copied into package");

        assert!(matches!(
            InstalledApplication::load(&inside_package, &fixture.package_root),
            Err(InstalledApplicationError::RecordInsidePackage)
        ));
        fixture.remove();
    }

    #[test]
    fn rejects_a_path_that_escapes_the_package_before_reading_it() {
        let fixture = fixture();
        let contents = fs::read_to_string(&fixture.record_path).expect("record is read");
        fs::write(
            &fixture.record_path,
            contents.replace("bin/sample.exe", "../sample.exe"),
        )
        .expect("record is changed");

        assert!(matches!(
            InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
            Err(InstalledApplicationError::InvalidExecutablePath)
        ));
        fixture.remove();
    }
}
