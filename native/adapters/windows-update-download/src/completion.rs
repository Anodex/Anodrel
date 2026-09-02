//! Fixed machine-policy postcondition proof for one completed update handoff.

use std::fmt;

use anodrel_windows_installer::PackageVersion;
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::VerifiedDownloadedInstaller;

/// An expected update image could not be proved as the selected machine release.
#[derive(Debug)]
pub enum UpdateCompletionError {
    /// The elevated installer reported a nonzero exit and is not accepted.
    InstallerReportedFailure,
    /// Fixed machine policy could not select one valid current application.
    InstalledPolicyInvalid(PolicyStoreError),
    /// Windows did not accept the selected executable's Authenticode signer.
    InstalledSignatureInvalid(SignatureError),
    /// The selected executable signer differed from its fixed machine record.
    InstalledPublisherMismatch,
    /// The selected record differed from the verified candidate identity.
    CandidateIdentityMismatch,
    /// The selected executable signer differed from the verified candidate.
    CandidatePublisherMismatch,
    /// The selected package directory did not have the candidate's exact version.
    CandidateVersionMismatch,
}

impl fmt::Display for UpdateCompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InstallerReportedFailure => "the elevated update did not complete successfully",
            Self::InstalledPolicyInvalid(_) => "the selected application policy is invalid",
            Self::InstalledSignatureInvalid(_) => {
                "Windows did not accept the selected executable signature"
            }
            Self::InstalledPublisherMismatch => {
                "the selected executable publisher does not match policy"
            }
            Self::CandidateIdentityMismatch => {
                "the selected application does not match the update candidate"
            }
            Self::CandidatePublisherMismatch => {
                "the selected application publisher does not match the update candidate"
            }
            Self::CandidateVersionMismatch => {
                "the selected application version does not match the update candidate"
            }
        })
    }
}

impl std::error::Error for UpdateCompletionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstalledPolicyInvalid(error) => Some(error),
            Self::InstalledSignatureInvalid(error) => Some(error),
            Self::InstallerReportedFailure
            | Self::InstalledPublisherMismatch
            | Self::CandidateIdentityMismatch
            | Self::CandidatePublisherMismatch
            | Self::CandidateVersionMismatch => None,
        }
    }
}

/// Opaque proof that fixed current policy selected the expected update release.
#[derive(Debug)]
pub struct VerifiedUpdateInstallation {
    _private: (),
}

/// Re-reads and proves the current selected application after an update handoff.
///
/// The caller must invoke this only after the fixed elevated installer reported
/// exit code zero. It revalidates the fixed machine record and selected
/// executable, then requires exact identity, publisher, and canonical version
/// continuity with the locked candidate. It performs no process, elevation,
/// network, cache, installation, policy mutation, cleanup, or restart action.
pub fn verify_current_update_selection(
    candidate: &VerifiedDownloadedInstaller,
) -> Result<VerifiedUpdateInstallation, UpdateCompletionError> {
    let manifest = candidate.manifest();
    let installed = load_installed_application(manifest.application_id())
        .map_err(UpdateCompletionError::InstalledPolicyInvalid)?;
    if installed.identity().application_id() != manifest.application_id() {
        return Err(UpdateCompletionError::CandidateIdentityMismatch);
    }
    let signer = verify_embedded_signature(installed.executable_path())
        .map_err(UpdateCompletionError::InstalledSignatureInvalid)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(UpdateCompletionError::InstalledPublisherMismatch);
    }
    if !manifest.matches_publisher_fingerprint(signer.as_bytes()) {
        return Err(UpdateCompletionError::CandidatePublisherMismatch);
    }
    let version = installed
        .package_root()
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(PackageVersion::from_canonical_directory_name);
    if version != Some(manifest.package_version()) {
        return Err(UpdateCompletionError::CandidateVersionMismatch);
    }
    Ok(VerifiedUpdateInstallation { _private: () })
}

#[cfg(test)]
mod tests {
    use super::UpdateCompletionError;

    #[test]
    fn postcondition_messages_do_not_expose_paths_or_policy_data() {
        assert_eq!(
            UpdateCompletionError::CandidateVersionMismatch.to_string(),
            "the selected application version does not match the update candidate"
        );
    }
}
