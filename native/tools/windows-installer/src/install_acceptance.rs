//! Opaque initial-install preflight and machine-policy postcondition proof.

use std::fmt;

use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{
    PackageVersion, SignedReleaseError, VerifiedEmbeddedRelease, verify_current_signed_release,
};

/// A signed current installer that may enter a later initial-install handoff.
pub struct PreparedInitialInstall {
    release: VerifiedEmbeddedRelease<'static>,
}

impl fmt::Debug for PreparedInitialInstall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PreparedInitialInstall(..)")
    }
}

impl PreparedInitialInstall {
    /// Returns the signed release version for the fixed native confirmation.
    #[must_use]
    pub const fn candidate_version(&self) -> PackageVersion {
        self.release.release().manifest().package_version()
    }
}

/// A safe failure while preparing a possible first installation.
#[derive(Debug)]
pub enum InitialInstallPreflightError {
    /// The current embedded installer release did not pass its signed gate.
    InstallerInvalid(SignedReleaseError),
    /// A selected policy already exists and requires the update path instead.
    ApplicationAlreadyInstalled,
    /// Existing machine policy could not be read safely before handoff.
    ExistingPolicyInvalid(PolicyStoreError),
}

impl fmt::Display for InitialInstallPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InstallerInvalid(_) => "the signed installer release is invalid",
            Self::ApplicationAlreadyInstalled => {
                "the application is already installed and requires the update path"
            }
            Self::ExistingPolicyInvalid(_) => {
                "the existing application policy cannot be used safely"
            }
        })
    }
}

impl std::error::Error for InitialInstallPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstallerInvalid(error) => Some(error),
            Self::ExistingPolicyInvalid(error) => Some(error),
            Self::ApplicationAlreadyInstalled => None,
        }
    }
}

/// A safe failure while proving the selected initial installation afterwards.
#[derive(Debug)]
pub enum InitialInstallCompletionError {
    /// The elevated installer process returned a nonzero exit code.
    InstallerReportedFailure,
    /// Fixed machine policy could not select one valid current application.
    InstalledPolicyInvalid(PolicyStoreError),
    /// Windows did not accept the selected executable's Authenticode signer.
    InstalledSignatureInvalid(SignatureError),
    /// The selected executable signer differed from its fixed machine record.
    InstalledPublisherMismatch,
    /// The selected record differed from the signed installer identity.
    InstallerIdentityMismatch,
    /// The selected executable signer differed from the signed installer.
    InstallerPublisherMismatch,
    /// The selected package root did not have the installer's exact version.
    InstallerVersionMismatch,
}

impl fmt::Display for InitialInstallCompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InstallerReportedFailure => "the installer reported that installation failed",
            Self::InstalledPolicyInvalid(_) => "the selected application policy is invalid",
            Self::InstalledSignatureInvalid(_) => {
                "Windows did not accept the selected executable signature"
            }
            Self::InstalledPublisherMismatch => {
                "the selected executable publisher does not match policy"
            }
            Self::InstallerIdentityMismatch => {
                "the selected application does not match the signed installer"
            }
            Self::InstallerPublisherMismatch => {
                "the selected application publisher does not match the signed installer"
            }
            Self::InstallerVersionMismatch => {
                "the selected application version does not match the signed installer"
            }
        })
    }
}

impl std::error::Error for InitialInstallCompletionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstallerReportedFailure => None,
            Self::InstalledPolicyInvalid(error) => Some(error),
            Self::InstalledSignatureInvalid(error) => Some(error),
            Self::InstalledPublisherMismatch
            | Self::InstallerIdentityMismatch
            | Self::InstallerPublisherMismatch
            | Self::InstallerVersionMismatch => None,
        }
    }
}

/// Opaque proof that fixed current policy selected the expected first release.
#[derive(Debug)]
pub struct VerifiedInitialInstallation {
    _private: (),
}

/// Prepares the current signed installer for a later initial-install handoff.
///
/// The embedded release must pass Windows Authenticode and its own release
/// contract. Its fixed identity must also have no selected machine policy. This
/// does not show a dialog, elevate, launch, install, write policy, or retain a
/// preference.
pub fn prepare_current_initial_install()
-> Result<PreparedInitialInstall, InitialInstallPreflightError> {
    let release =
        verify_current_signed_release().map_err(InitialInstallPreflightError::InstallerInvalid)?;
    require_no_selected_policy(release.release().manifest().application_id())?;
    Ok(PreparedInitialInstall { release })
}

/// Proves fixed policy selected a prepared installer release after a zero exit.
///
/// Call this only after a separate elevated fixed `install` process reported
/// success. The check revalidates the selected record and executable, requiring
/// exact identity, publisher, and canonical package-version continuity. It does
/// not inspect a process, elevate, write policy, launch an application, or
/// report an installer exit outcome.
pub fn verify_current_initial_installation(
    installer: &PreparedInitialInstall,
) -> Result<VerifiedInitialInstallation, InitialInstallCompletionError> {
    let manifest = installer.release.release().manifest();
    let installed = load_installed_application(manifest.application_id())
        .map_err(InitialInstallCompletionError::InstalledPolicyInvalid)?;
    if installed.identity().application_id() != manifest.application_id() {
        return Err(InitialInstallCompletionError::InstallerIdentityMismatch);
    }
    let signer = verify_embedded_signature(installed.executable_path())
        .map_err(InitialInstallCompletionError::InstalledSignatureInvalid)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(InitialInstallCompletionError::InstalledPublisherMismatch);
    }
    if !manifest.matches_publisher_fingerprint(signer.as_bytes()) {
        return Err(InitialInstallCompletionError::InstallerPublisherMismatch);
    }
    let version = installed
        .package_root()
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(PackageVersion::from_canonical_directory_name);
    if version != Some(manifest.package_version()) {
        return Err(InitialInstallCompletionError::InstallerVersionMismatch);
    }
    Ok(VerifiedInitialInstallation { _private: () })
}

fn require_no_selected_policy(application_id: &str) -> Result<(), InitialInstallPreflightError> {
    match load_installed_application(application_id) {
        Err(PolicyStoreError::RecordNotFound) => Ok(()),
        Ok(_) => Err(InitialInstallPreflightError::ApplicationAlreadyInstalled),
        Err(error) => Err(InitialInstallPreflightError::ExistingPolicyInvalid(error)),
    }
}

#[cfg(test)]
mod tests {
    use anodrel_windows_policy::PolicyStoreError;
    use anodrel_windows_signature::SignatureError;

    use super::{
        InitialInstallCompletionError, InitialInstallPreflightError,
        prepare_current_initial_install, require_no_selected_policy,
    };
    use crate::SignedReleaseError;

    #[test]
    fn an_unsigned_current_installer_cannot_reach_initial_install_handoff() {
        assert!(matches!(
            prepare_current_initial_install(),
            Err(InitialInstallPreflightError::InstallerInvalid(
                SignedReleaseError::SignatureInvalid(SignatureError::TrustRejected)
            ))
        ));
    }

    #[test]
    fn an_invalid_identity_stops_before_a_policy_or_uac_route() {
        assert!(matches!(
            require_no_selected_policy("org.anodrel/escape"),
            Err(InitialInstallPreflightError::ExistingPolicyInvalid(
                PolicyStoreError::InvalidApplicationId
            ))
        ));
    }

    #[test]
    fn completion_failures_keep_machine_details_private() {
        assert_eq!(
            InitialInstallCompletionError::InstallerVersionMismatch.to_string(),
            "the selected application version does not match the signed installer"
        );
        assert_eq!(
            InitialInstallCompletionError::InstallerReportedFailure.to_string(),
            "the installer reported that installation failed"
        );
    }
}
