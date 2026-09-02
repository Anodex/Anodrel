//! Locked external-installer image verification before an elevated handoff.

use std::{fmt, path::Path};

use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{
    EmbeddedReleaseError, ReleaseManifest,
    resources::{LockedEmbeddedRelease, lock_and_read_release},
};

/// A downloaded installer image that is signed and held against writes.
pub struct VerifiedInstallerImage {
    release: LockedEmbeddedRelease,
}

impl VerifiedInstallerImage {
    /// Returns the checked release facts from the still-locked image.
    #[must_use]
    pub fn manifest(&self) -> &ReleaseManifest {
        self.release.manifest()
    }
}

impl fmt::Debug for VerifiedInstallerImage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedInstallerImage(..)")
    }
}

/// A candidate installer image could not be accepted for a later handoff.
#[derive(Debug)]
pub enum InstallerImageError {
    /// The image could not supply a complete valid owned release.
    ReleaseInvalid(EmbeddedReleaseError),
    /// Windows did not accept the image Authenticode signature.
    SignatureInvalid(SignatureError),
    /// The accepted signer differed from the embedded release publisher.
    PublisherMismatch,
}

impl fmt::Display for InstallerImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ReleaseInvalid(_) => "the downloaded installer release is invalid",
            Self::SignatureInvalid(_) => {
                "Windows did not accept the downloaded installer signature"
            }
            Self::PublisherMismatch => {
                "the downloaded installer publisher does not match its release"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for InstallerImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReleaseInvalid(error) => Some(error),
            Self::SignatureInvalid(error) => Some(error),
            Self::PublisherMismatch => None,
        }
    }
}

/// Locks and verifies one downloaded installer image without executing it.
///
/// Windows maps the absolute image for resources with exclusive write
/// protection before its release resources and Authenticode signer are checked.
/// The returned opaque value holds that mapping until launch or disposal. This
/// performs no catalogue comparison, elevation, process launch, installation,
/// policy read, or machine mutation.
pub fn verify_locked_installer_image(
    path: &Path,
) -> Result<VerifiedInstallerImage, InstallerImageError> {
    let release = lock_and_read_release(path).map_err(InstallerImageError::ReleaseInvalid)?;
    let signer = verify_embedded_signature(path).map_err(InstallerImageError::SignatureInvalid)?;
    if !release
        .manifest()
        .matches_publisher_fingerprint(signer.as_bytes())
    {
        return Err(InstallerImageError::PublisherMismatch);
    }
    Ok(VerifiedInstallerImage { release })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{EmbeddedReleaseError, InstallerImageError, verify_locked_installer_image};

    #[test]
    fn a_relative_image_cannot_enter_the_locked_release_gate() {
        assert!(matches!(
            verify_locked_installer_image(Path::new("candidate.exe")),
            Err(InstallerImageError::ReleaseInvalid(
                EmbeddedReleaseError::ImageUnavailable
            ))
        ));
    }
}
