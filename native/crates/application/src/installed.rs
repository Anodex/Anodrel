//! Strict installed application-record validation.
//!
//! The record is selected by a host-controlled policy store outside the
//! application package. This module validates that record's package and
//! executable binding but does not select policy storage, verify Authenticode,
//! launch a process, or expose policy data to an application surface.

use std::{
    collections::BTreeMap,
    fmt, fs,
    io::Read,
    path::{Path, PathBuf},
};

use anodrel_json::JsonValue;
use anodrel_network::NetworkOriginPolicy;
use anodrel_protocol::Capability;

use crate::{
    ApplicationError, ApplicationIdentity, ApplicationPackage, MAX_EXECUTABLE_BYTES,
    MAX_INSTALL_RECORD_BYTES, manifest, sha256,
};

const PACKAGE_MANIFEST_NAME: &str = "anodrel.application.json";

mod filesystem;
mod launcher;
mod product;
mod record;
mod start_menu_name;
mod update_catalogue;

pub(super) use filesystem::canonical_executable_path;
pub use product::{ProductDisplayMetadata, ProductDisplayMetadataError};
pub use start_menu_name::{StartMenuName, StartMenuNameError};
pub use update_catalogue::{
    MAX_UPDATE_CATALOGUE_PATH_BYTES, UpdateCatalogueLocation, UpdateCatalogueLocationError,
};

use filesystem::{canonical_directory, digest_file, read_limited};

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
    update_catalogue: Option<UpdateCatalogueLocation>,
    product_metadata: Option<ProductDisplayMetadata>,
    start_menu_name: Option<StartMenuName>,
    product_launcher: Option<launcher::ProductLauncher>,
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

        let record = record::parse(&read_limited(&record_path, MAX_INSTALL_RECORD_BYTES)?)?;
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
        let record = record::parse(record)?;
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

    /// Returns the canonical package root for a private native host operation.
    ///
    /// This path must never be serialized to an application, renderer, or
    /// protocol response.
    #[must_use]
    pub fn package_root(&self) -> &Path {
        &self.package_root
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

    /// Returns the signed update-catalogue location, when this release opted in.
    ///
    /// This remains private host-composition data. It is not application network
    /// authority and must never be serialized into a protocol response.
    #[must_use]
    pub fn update_catalogue_location(&self) -> Option<&UpdateCatalogueLocation> {
        self.update_catalogue.as_ref()
    }

    /// Returns selected signed display metadata for a host-owned product surface.
    ///
    /// This is private native-host composition data and is never serialized into
    /// an application protocol response.
    #[must_use]
    pub fn product_metadata(&self) -> Option<&ProductDisplayMetadata> {
        self.product_metadata.as_ref()
    }

    /// Returns the signed Windows-safe Start-menu link name, when selected.
    ///
    /// This remains private native-host composition data. It is never an
    /// application protocol response or a general filesystem path.
    #[must_use]
    pub fn start_menu_name(&self) -> Option<&StartMenuName> {
        self.start_menu_name.as_ref()
    }

    /// Returns the selected product launcher for private Windows host composition.
    ///
    /// This never reaches an application protocol response or general process
    /// interface. A record without version 1.23 product-launch data returns no
    /// launcher and cannot become a Start-menu target.
    #[must_use]
    pub fn product_launcher_path(&self) -> Option<&Path> {
        self.product_launcher
            .as_ref()
            .map(launcher::ProductLauncher::path)
    }

    /// Rechecks the selected product launcher through a caller-held file lock.
    ///
    /// The caller must compare the resulting canonical path with its current
    /// executable before it creates a product window. This is private host
    /// composition, not an application process API.
    pub fn revalidate_product_launcher<R: Read>(
        &self,
        path: &Path,
        reader: &mut R,
    ) -> Result<(), InstalledApplicationError> {
        self.product_launcher
            .as_ref()
            .ok_or(InstalledApplicationError::ProductLauncherUnavailable)?
            .revalidate(path, reader)
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
    record: record::ParsedRecord,
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
    let product_launcher = record
        .product_launcher
        .map(|(path, digest)| {
            launcher::ProductLauncher::from_record(&package_root, &executable_path, &path, digest)
        })
        .transpose()?;

    Ok(InstalledApplication {
        identity: package.identity().clone(),
        package_root,
        executable_path,
        executable_digest: record.executable_digest,
        publisher_fingerprint: PublisherFingerprint(record.publisher_fingerprint),
        capabilities: record.capabilities,
        network_policy: record.network_policy,
        update_catalogue: record.update_catalogue,
        product_metadata: record.product_metadata,
        start_menu_name: record.start_menu_name,
        product_launcher,
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
    ProductLauncherUnavailable,
    ProductLauncherMatchesApplication,
    ProductLauncherPathChanged,
    ProductLauncherDigestMismatch,
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
            Self::ProductLauncherUnavailable => {
                "installed application does not declare a product launcher"
            }
            Self::ProductLauncherMatchesApplication => {
                "installed application product launcher is not distinct"
            }
            Self::ProductLauncherPathChanged => {
                "installed application product launcher changed after policy validation"
            }
            Self::ProductLauncherDigestMismatch => {
                "installed application product launcher digest does not match"
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

#[cfg(test)]
mod tests;
