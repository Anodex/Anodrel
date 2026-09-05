//! Current installed-policy preflight for a CMS-verified catalogue candidate.

use std::fmt;

use anodrel_update_catalogue::UpdateInstaller;
use anodrel_windows_installer::{PackageVersion, VerifiedInstallerImage};
use anodrel_windows_policy::load_installed_application;
use anodrel_windows_signature::verify_embedded_signature;
use anodrel_windows_update_catalogue_signature::VerifiedUpdateCatalogue;

use crate::UpdateDownloadError;

/// One CMS-verified catalogue that still matches the current installed release.
pub struct PreparedUpdateDownload {
    catalogue: VerifiedUpdateCatalogue,
}

impl PreparedUpdateDownload {
    pub(crate) fn installer(&self) -> &UpdateInstaller {
        self.catalogue.installer()
    }

    /// Returns the CMS-verified candidate version for native host presentation.
    #[must_use]
    pub const fn candidate_version(&self) -> PackageVersion {
        self.catalogue.package_version()
    }

    /// Returns the signed exact byte total for host-owned transfer progress.
    #[must_use]
    pub fn installer_byte_length(&self) -> u64 {
        self.catalogue.installer().byte_length()
    }

    pub(crate) fn matches_image(&self, image: &VerifiedInstallerImage) -> bool {
        self.catalogue.matches_release(image.manifest())
    }
}

impl fmt::Debug for PreparedUpdateDownload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PreparedUpdateDownload(..)")
    }
}

/// Rechecks the current installed release before permitting one image transfer.
///
/// The input has already passed attached-CMS verification for one exact
/// publisher. This function then reads only the matching fixed machine policy,
/// verifies the selected executable with Windows Authenticode, and requires
/// exact application, publisher, and strictly newer canonical-version facts.
/// It performs no network or file-cache operation.
pub fn prepare_current_update_download(
    catalogue: VerifiedUpdateCatalogue,
) -> Result<PreparedUpdateDownload, UpdateDownloadError> {
    let application_id = catalogue.application_id().to_owned();
    let installed = load_installed_application(&application_id)
        .map_err(UpdateDownloadError::InstalledPolicyInvalid)?;
    let signer = verify_embedded_signature(installed.executable_path())
        .map_err(UpdateDownloadError::InstalledSignatureInvalid)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(UpdateDownloadError::InstalledPublisherMismatch);
    }
    let installed_version = installed
        .package_root()
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(PackageVersion::from_canonical_directory_name)
        .ok_or(UpdateDownloadError::InstalledVersionInvalid)?;
    if !catalogue.matches_installed(&application_id, signer.as_bytes()) {
        return Err(UpdateDownloadError::CandidateIdentityMismatch);
    }
    if !catalogue.is_newer_than(installed_version) {
        return Err(UpdateDownloadError::CandidateNotNewer);
    }
    Ok(PreparedUpdateDownload { catalogue })
}
