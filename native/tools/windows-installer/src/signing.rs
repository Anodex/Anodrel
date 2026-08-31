//! Current-installer Authenticode activation gate.

use std::fmt;

use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{EmbeddedRelease, EmbeddedReleaseError, read_current_release};

/// A signed installer release could not be activated safely.
#[derive(Debug)]
pub enum SignedReleaseError {
    /// The current installer executable path could not be obtained.
    CurrentImagePathUnavailable,
    /// Windows did not accept the current installer executable signature.
    SignatureInvalid(SignatureError),
    /// Fixed release resources were absent or did not meet their contract.
    ResourcesInvalid(EmbeddedReleaseError),
    /// The accepted installer signer differed from the embedded publisher.
    PublisherMismatch,
}

impl fmt::Display for SignedReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CurrentImagePathUnavailable => "the current installer image is unavailable",
            Self::SignatureInvalid(_) => "Windows did not accept the installer signature",
            Self::ResourcesInvalid(_) => "the installer release resources are invalid",
            Self::PublisherMismatch => "the installer publisher does not match its release",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SignedReleaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SignatureInvalid(error) => Some(error),
            Self::ResourcesInvalid(error) => Some(error),
            Self::CurrentImagePathUnavailable | Self::PublisherMismatch => None,
        }
    }
}

/// A release that passed both current-installer and embedded-data verification.
#[derive(Debug)]
pub struct VerifiedEmbeddedRelease<'image> {
    release: EmbeddedRelease<'image>,
}

impl<'image> VerifiedEmbeddedRelease<'image> {
    /// Returns the checked release selected from the signed current installer.
    #[must_use]
    pub const fn release(&self) -> &EmbeddedRelease<'image> {
        &self.release
    }
}

/// Activates the release only when the current installer signer and manifest agree.
///
/// This operation intentionally has no installation side effect. A later install
/// operation must accept only this checked value, then perform its own staged
/// filesystem and machine-policy transaction.
pub fn verify_current_signed_release()
-> Result<VerifiedEmbeddedRelease<'static>, SignedReleaseError> {
    let signer = verify_current_image_signer()?;
    let release = read_current_release().map_err(SignedReleaseError::ResourcesInvalid)?;
    if !release
        .manifest()
        .matches_publisher_fingerprint(signer.as_bytes())
    {
        return Err(SignedReleaseError::PublisherMismatch);
    }
    Ok(VerifiedEmbeddedRelease { release })
}

fn verify_current_image_signer()
-> Result<anodrel_windows_signature::SignerFingerprint, SignedReleaseError> {
    let path =
        std::env::current_exe().map_err(|_| SignedReleaseError::CurrentImagePathUnavailable)?;
    verify_embedded_signature(&path).map_err(SignedReleaseError::SignatureInvalid)
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{SignedReleaseError, verify_current_image_signer, verify_current_signed_release};

    #[test]
    fn a_normal_unsigned_test_image_cannot_activate_a_release() {
        assert!(matches!(
            verify_current_image_signer(),
            Err(SignedReleaseError::SignatureInvalid(
                SignatureError::TrustRejected
            ))
        ));
        assert!(matches!(
            verify_current_signed_release(),
            Err(SignedReleaseError::SignatureInvalid(
                SignatureError::TrustRejected
            ))
        ));
    }
}
