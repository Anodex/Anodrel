//! Signed update-candidate preflight with publisher continuity and rollback protection.

use std::fmt;

use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{PackageVersion, SignedReleaseError, verify_current_signed_release};

/// A strictly newer signed release eligible for a later owned update transaction.
pub struct VerifiedUpdateCandidate {
    application_id: String,
    package_version: PackageVersion,
}

impl fmt::Debug for VerifiedUpdateCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedUpdateCandidate")
            .field("application_id", &self.application_id)
            .field("package_version", &self.package_version)
            .finish_non_exhaustive()
    }
}

/// A signed release could not become a safe update candidate.
#[derive(Debug)]
pub enum UpdatePreflightError {
    /// The current candidate installer did not pass its signed embedded-release gate.
    CandidateInvalid(SignedReleaseError),
    /// The fixed installed application record was missing or invalid.
    InstalledPolicyInvalid(PolicyStoreError),
    /// Windows did not accept the installed executable signature.
    InstalledSignatureInvalid(SignatureError),
    /// The installed executable signer differed from its validated machine record.
    InstalledPublisherMismatch,
    /// The installed package root did not use the owned canonical version name.
    InstalledVersionInvalid,
    /// The candidate publisher differed from the installed executable publisher.
    CandidatePublisherMismatch,
    /// The candidate version was equal to or older than the selected version.
    CandidateNotNewer,
}

impl fmt::Display for UpdatePreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CandidateInvalid(_) => "the signed update candidate is invalid",
            Self::InstalledPolicyInvalid(_) => "the installed application policy is invalid",
            Self::InstalledSignatureInvalid(_) => {
                "Windows did not accept the installed executable signature"
            }
            Self::InstalledPublisherMismatch => {
                "the installed executable publisher does not match policy"
            }
            Self::InstalledVersionInvalid => {
                "the installed package root does not use an owned version name"
            }
            Self::CandidatePublisherMismatch => {
                "the update candidate publisher does not match the installed application"
            }
            Self::CandidateNotNewer => {
                "the update candidate is not newer than the installed release"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UpdatePreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CandidateInvalid(error) => Some(error),
            Self::InstalledPolicyInvalid(error) => Some(error),
            Self::InstalledSignatureInvalid(error) => Some(error),
            Self::InstalledPublisherMismatch
            | Self::InstalledVersionInvalid
            | Self::CandidatePublisherMismatch
            | Self::CandidateNotNewer => None,
        }
    }
}

/// Verifies the current signed installer as a strictly newer update candidate.
///
/// The candidate and the selected installed executable must have the same
/// accepted publisher. The selected package root must end in Anodrel's exact
/// canonical release-version name, and the candidate version must be strictly
/// greater. This operation does not download, stage, promote, publish, launch,
/// modify policy, or expose a package path to an application.
pub fn verify_current_update_candidate() -> Result<VerifiedUpdateCandidate, UpdatePreflightError> {
    let candidate =
        verify_current_signed_release().map_err(UpdatePreflightError::CandidateInvalid)?;
    let manifest = candidate.release().manifest();
    let installed = load_installed_application(manifest.application_id())
        .map_err(UpdatePreflightError::InstalledPolicyInvalid)?;
    let signer = verify_embedded_signature(installed.executable_path())
        .map_err(UpdatePreflightError::InstalledSignatureInvalid)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(UpdatePreflightError::InstalledPublisherMismatch);
    }
    if !manifest.matches_publisher_fingerprint(signer.as_bytes()) {
        return Err(UpdatePreflightError::CandidatePublisherMismatch);
    }
    let installed_version = package_version_from_root(installed.package_root())
        .ok_or(UpdatePreflightError::InstalledVersionInvalid)?;
    let package_version = manifest.package_version();
    if package_version <= installed_version {
        return Err(UpdatePreflightError::CandidateNotNewer);
    }
    Ok(VerifiedUpdateCandidate {
        application_id: manifest.application_id().to_owned(),
        package_version,
    })
}

fn package_version_from_root(root: &std::path::Path) -> Option<PackageVersion> {
    root.file_name()?
        .to_str()
        .and_then(PackageVersion::from_directory_name)
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{UpdatePreflightError, package_version_from_root, verify_current_update_candidate};
    use crate::SignedReleaseError;

    #[test]
    fn parses_only_canonical_owned_version_directory_names() {
        let version =
            package_version_from_root(std::path::Path::new(r"C:\\Program Files\\Anodrel\\1.2.3"))
                .expect("the canonical name parses");
        assert_eq!(
            (version.major(), version.minor(), version.patch()),
            (1, 2, 3)
        );
        for invalid in ["01.2.3", "1.02.3", "1.2.003", "1.2", "1.2.3.4", "1.-2.3"] {
            assert!(
                package_version_from_root(std::path::Path::new(invalid)).is_none(),
                "{invalid} must not be an owned version name"
            );
        }
    }

    #[test]
    fn an_unsigned_current_installer_cannot_be_an_update_candidate() {
        assert!(matches!(
            verify_current_update_candidate(),
            Err(UpdatePreflightError::CandidateInvalid(
                SignedReleaseError::SignatureInvalid(SignatureError::TrustRejected)
            ))
        ));
    }
}
