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
    executable_path: PathBuf,
    executable_digest: [u8; 32],
    publisher_fingerprint: PublisherFingerprint,
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
        let package_root = canonical_directory(Path::new(&record.package_root))
            .map_err(|_| InstalledApplicationError::InvalidPackageRoot)?;
        if record_path.starts_with(&package_root) {
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

        Ok(Self {
            identity: package.identity().clone(),
            executable_path,
            executable_digest: record.executable_digest,
            publisher_fingerprint: PublisherFingerprint(record.publisher_fingerprint),
        })
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
}

impl fmt::Debug for InstalledApplication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InstalledApplication(..)")
    }
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
}

fn parse_record(input: &[u8]) -> Result<ParsedRecord, InstalledApplicationError> {
    let input = std::str::from_utf8(input).map_err(|_| InstalledApplicationError::InvalidRecord)?;
    let root = JsonValue::parse(input).map_err(|_| InstalledApplicationError::InvalidRecord)?;
    let fields = root
        .as_object()
        .ok_or(InstalledApplicationError::InvalidRecord)?;
    exact_fields(
        fields,
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
        ],
    )?;
    validate_version(required_object(fields, "recordVersion")?)?;

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

    Ok(ParsedRecord {
        application_id: application_id.to_owned(),
        package_root: package_root.to_owned(),
        executable_path: executable_path.to_owned(),
        executable_digest,
        publisher_fingerprint,
    })
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

fn read_limited(path: &Path, maximum: usize) -> Result<Vec<u8>, InstalledApplicationError> {
    let file = File::open(path).map_err(InstalledApplicationError::Io)?;
    let mut reader = file.take((maximum + 1) as u64);
    let mut contents = Vec::with_capacity(maximum.min(4_096));
    reader
        .read_to_end(&mut contents)
        .map_err(InstalledApplicationError::Io)?;
    if contents.len() > maximum {
        Err(InstalledApplicationError::RecordTooLarge)
    } else {
        Ok(contents)
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

fn validate_version(fields: &BTreeMap<String, JsonValue>) -> Result<(), InstalledApplicationError> {
    exact_fields(fields, &["major", "minor"])?;
    let major = fields
        .get("major")
        .and_then(JsonValue::as_u16)
        .ok_or(InstalledApplicationError::InvalidRecord)?;
    let minor = fields
        .get("minor")
        .and_then(JsonValue::as_u16)
        .ok_or(InstalledApplicationError::InvalidRecord)?;
    if major == 1 && minor == 0 {
        Ok(())
    } else {
        Err(InstalledApplicationError::InvalidRecord)
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
