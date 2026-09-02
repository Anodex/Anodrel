//! Locked exact-release acceptance for a digest-checked installer image.

use std::{fmt, path::Path};

use anodrel_windows_installer::{
    InstallerImageError, VerifiedInstallerImage, verify_locked_installer_image,
};

use crate::{DownloadedInstaller, PreparedUpdateDownload};

/// A downloaded image that can safely enter the later elevated handoff.
pub struct VerifiedDownloadedInstaller {
    // Field order intentionally drops the locked mapping before the private
    // downloaded-file owner tries to remove the source image.
    image: VerifiedInstallerImage,
    downloaded: DownloadedInstaller,
}

impl VerifiedDownloadedInstaller {
    /// Returns the private absolute installer path for the direct handoff only.
    ///
    /// This remains host-only data and must never become an application,
    /// renderer, protocol, command-line, or environment value.
    #[must_use]
    pub fn path(&self) -> &Path {
        let _ = &self.image;
        self.downloaded.path()
    }

    /// Retains the private image for later owned recovery after a running handoff.
    ///
    /// This does not expose or launch the image. It prevents this value from
    /// deleting a file that a separately owned elevated process may still hold.
    pub fn retain_for_recovery(&mut self) {
        self.downloaded.retain_for_recovery();
    }

    pub(crate) fn manifest(&self) -> &anodrel_windows_installer::ReleaseManifest {
        self.image.manifest()
    }
}

impl fmt::Debug for VerifiedDownloadedInstaller {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedDownloadedInstaller(..)")
    }
}

/// A digest-checked download could not become a locked exact release image.
#[derive(Debug)]
pub enum UpdateImageAcceptanceError {
    /// The candidate image could not pass its locked signed-release gate.
    ImageInvalid(InstallerImageError),
    /// The accepted image release differed from the CMS-verified candidate.
    CandidateMismatch,
}

impl fmt::Display for UpdateImageAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ImageInvalid(_) => "the downloaded update installer is invalid",
            Self::CandidateMismatch => {
                "the downloaded update installer does not match its signed catalogue"
            }
        })
    }
}

impl std::error::Error for UpdateImageAcceptanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ImageInvalid(error) => Some(error),
            Self::CandidateMismatch => None,
        }
    }
}

/// Locks and rechecks one downloaded installer before elevation can use it.
///
/// The file must already have passed the exact signed byte descriptor through
/// `DownloadedInstaller`. This function maps it without running code, verifies
/// Windows Authenticode and the embedded full release, then requires the same
/// identity, version, and publisher as the CMS-verified prepared candidate. It
/// performs no network, cache selection, elevation, process launch, policy
/// mutation, or installation.
pub fn verify_downloaded_update_image(
    prepared: &PreparedUpdateDownload,
    downloaded: DownloadedInstaller,
) -> Result<VerifiedDownloadedInstaller, UpdateImageAcceptanceError> {
    let image = verify_locked_installer_image(downloaded.path())
        .map_err(UpdateImageAcceptanceError::ImageInvalid)?;
    if !prepared.matches_image(&image) {
        return Err(UpdateImageAcceptanceError::CandidateMismatch);
    }
    Ok(VerifiedDownloadedInstaller { image, downloaded })
}

#[cfg(test)]
mod tests {
    use super::UpdateImageAcceptanceError;

    #[test]
    fn image_acceptance_messages_do_not_include_native_paths_or_statuses() {
        assert_eq!(
            UpdateImageAcceptanceError::CandidateMismatch.to_string(),
            "the downloaded update installer does not match its signed catalogue"
        );
    }
}
