//! Private filesystem staging for checked Windows release bundles.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anodrel_application::{InstalledApplication, InstalledApplicationError, sha256};
use anodrel_release_bundle::{BundleEntry, ReleaseBundle};

use crate::ReleaseManifest;

mod path;

const STAGE_ATTEMPTS: usize = 32;
static NEXT_STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A checked package in a private, not-yet-published staging directory.
///
/// The directory is removed when this value is dropped. A later promotion
/// transaction must move it to a final version directory before publishing any
/// registry policy.
pub(crate) struct StagedRelease {
    package_root: PathBuf,
    executable_path: PathBuf,
    product_launcher_path: Option<PathBuf>,
    install_record: String,
    cleanup_on_drop: bool,
}

impl StagedRelease {
    /// Returns the private stage root for an owned later promotion transaction.
    #[must_use]
    pub(crate) fn package_root(&self) -> &Path {
        &self.package_root
    }

    /// Returns the already validated contained executable for signer verification.
    #[must_use]
    pub(crate) fn executable_path(&self) -> &Path {
        &self.executable_path
    }

    /// Returns the validated product launcher when this release declares one.
    #[must_use]
    pub(crate) fn product_launcher_path(&self) -> Option<&Path> {
        self.product_launcher_path.as_deref()
    }

    /// Transfers the retained record after a successful directory promotion.
    pub(crate) fn into_promoted_parts(mut self, package_root: PathBuf) -> (PathBuf, String) {
        self.cleanup_on_drop = false;
        (package_root, std::mem::take(&mut self.install_record))
    }
}

impl fmt::Debug for StagedRelease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StagedRelease")
            .field("install_record_bytes", &self.install_record.len())
            .finish_non_exhaustive()
    }
}

impl Drop for StagedRelease {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let _ = fs::remove_dir_all(&self.package_root);
        }
    }
}

/// A checked release could not become a private staged application package.
#[derive(Debug)]
pub enum StagedReleaseError {
    /// The installer-owned application staging parent was not an absolute directory.
    StagingParentInvalid,
    /// The installer could not allocate a new private staging directory.
    StagingCreationFailed,
    /// A checked bundle path cannot be represented safely on Windows.
    BundlePathInvalid,
    /// A new regular package file could not be written and synchronized.
    FileWriteFailed,
    /// A file did not retain the exact checked bytes after it was written.
    FileVerificationFailed,
    /// The staged package or rendered installed record did not validate.
    PackageInvalid(InstalledApplicationError),
}

impl fmt::Display for StagedReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::StagingParentInvalid => "the installer staging parent is invalid",
            Self::StagingCreationFailed => {
                "a private release staging directory could not be created"
            }
            Self::BundlePathInvalid => "a release bundle path is unsafe for Windows staging",
            Self::FileWriteFailed => "a staged release file could not be written",
            Self::FileVerificationFailed => "a staged release file did not verify",
            Self::PackageInvalid(_) => "the staged application package is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for StagedReleaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PackageInvalid(error) => Some(error),
            Self::StagingParentInvalid
            | Self::StagingCreationFailed
            | Self::BundlePathInvalid
            | Self::FileWriteFailed
            | Self::FileVerificationFailed => None,
        }
    }
}

/// Extracts a checked release into a new private staging directory.
///
/// `staging_parent` is installer-owned machine data, never a path selected by a
/// release, application, or command line. This boundary creates no version
/// directory, registry value, process, trust, or network connection. Callers
/// must first establish the installer signature boundary for the checked
/// manifest and bundle they supply.
pub(crate) fn stage_checked_release(
    staging_parent: &Path,
    manifest: &ReleaseManifest,
    bundle: &ReleaseBundle<'_>,
) -> Result<StagedRelease, StagedReleaseError> {
    let parent = canonical_staging_parent(staging_parent)?;
    if bundle.file(manifest.executable_path()).is_none() {
        return Err(StagedReleaseError::BundlePathInvalid);
    }
    if manifest
        .product_launcher()
        .is_some_and(|launcher| bundle.file(launcher.path()).is_none())
    {
        return Err(StagedReleaseError::BundlePathInvalid);
    }

    let staging = create_staging_directory(&parent, manifest)?;
    let guard = StagingGuard::new(staging);
    write_bundle(guard.path(), bundle)?;
    let record = manifest.render_install_record(guard.path());
    let installed =
        InstalledApplication::load_from_trusted_record(&record, manifest.application_id())
            .map_err(StagedReleaseError::PackageInvalid)?;

    Ok(guard.finish(
        record,
        installed.executable_path().to_path_buf(),
        installed.product_launcher_path().map(Path::to_path_buf),
    ))
}

fn canonical_staging_parent(path: &Path) -> Result<PathBuf, StagedReleaseError> {
    if !path.is_absolute() {
        return Err(StagedReleaseError::StagingParentInvalid);
    }
    let path = fs::canonicalize(path).map_err(|_| StagedReleaseError::StagingParentInvalid)?;
    let metadata = fs::metadata(&path).map_err(|_| StagedReleaseError::StagingParentInvalid)?;
    metadata
        .is_dir()
        .then_some(path)
        .ok_or(StagedReleaseError::StagingParentInvalid)
}

fn create_staging_directory(
    parent: &Path,
    manifest: &ReleaseManifest,
) -> Result<PathBuf, StagedReleaseError> {
    for _ in 0..STAGE_ATTEMPTS {
        let sequence = NEXT_STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let version = manifest.package_version();
        let name = format!(
            ".anodrel-stage-{}-{}-{}-{}-{sequence}",
            version.major(),
            version.minor(),
            version.patch(),
            std::process::id(),
        );
        let candidate = parent.join(name);
        match fs::create_dir(&candidate) {
            Ok(()) => return verify_new_staging_directory(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(StagedReleaseError::StagingCreationFailed),
        }
    }
    Err(StagedReleaseError::StagingCreationFailed)
}

fn verify_new_staging_directory(path: PathBuf) -> Result<PathBuf, StagedReleaseError> {
    let metadata =
        fs::symlink_metadata(&path).map_err(|_| StagedReleaseError::StagingCreationFailed)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(StagedReleaseError::StagingCreationFailed);
    }
    Ok(path)
}

fn write_bundle(root: &Path, bundle: &ReleaseBundle<'_>) -> Result<(), StagedReleaseError> {
    for entry in bundle.entries() {
        write_entry(root, entry)?;
    }
    Ok(())
}

fn write_entry(root: &Path, entry: &BundleEntry<'_>) -> Result<(), StagedReleaseError> {
    let output =
        path::output_path(root, entry.path()).ok_or(StagedReleaseError::BundlePathInvalid)?;
    let parent = output
        .parent()
        .ok_or(StagedReleaseError::BundlePathInvalid)?;
    path::create_private_directories(root, parent)
        .map_err(|_| StagedReleaseError::BundlePathInvalid)?;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|_| StagedReleaseError::FileWriteFailed)?;
    file.write_all(entry.contents())
        .and_then(|()| file.sync_all())
        .map_err(|_| StagedReleaseError::FileWriteFailed)?;
    drop(file);

    let mut written =
        File::open(&output).map_err(|_| StagedReleaseError::FileVerificationFailed)?;
    let Some((digest, length)) =
        sha256::digest_reader_limited(&mut written, entry.contents().len())
            .map_err(|_| StagedReleaseError::FileVerificationFailed)?
    else {
        return Err(StagedReleaseError::FileVerificationFailed);
    };
    let expected = sha256::digest(entry.contents());
    (length == entry.contents().len() && digest == expected)
        .then_some(())
        .ok_or(StagedReleaseError::FileVerificationFailed)
}

struct StagingGuard {
    path: PathBuf,
    retain: bool,
}

impl StagingGuard {
    const fn new(path: PathBuf) -> Self {
        Self {
            path,
            retain: false,
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn finish(
        mut self,
        install_record: String,
        executable_path: PathBuf,
        product_launcher_path: Option<PathBuf>,
    ) -> StagedRelease {
        self.retain = true;
        StagedRelease {
            package_root: self.path.clone(),
            executable_path,
            product_launcher_path,
            install_record,
            cleanup_on_drop: true,
        }
    }
}

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if !self.retain {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
mod tests;
