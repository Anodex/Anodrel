//! One no-argument composition of the owned signed machine-installation gates.

use std::fmt;

use anodrel_windows_policy::{PolicyStoreError, load_installed_application};

use crate::machine_root::current_machine_application_root;
use crate::{
    MachineRootError, PreparedReleaseError, PromotionError, PublicationError, SignedReleaseError,
    prepare_current_signed_release, promote_prepared_release, publish_promoted_release,
    verify_current_signed_release,
};

/// A release selected by the fixed machine policy through this installation transaction.
pub struct InstalledRelease {
    published: crate::PublishedRelease,
}

impl fmt::Debug for InstalledRelease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("InstalledRelease")
            .field(&self.published)
            .finish()
    }
}

/// The current signed installer release could not complete machine installation.
#[derive(Debug)]
pub enum InstallCurrentError {
    /// The current executable did not pass its signed embedded-release gate.
    InstallerInvalid(SignedReleaseError),
    /// A selected application policy already exists and must use the update path.
    ApplicationAlreadyInstalled,
    /// An existing machine policy could not be read safely before installation.
    ExistingPolicyInvalid(PolicyStoreError),
    /// The fixed machine root could not be established from the signed identity.
    MachineRootInvalid(MachineRootError),
    /// The signed release could not become a prepared private stage.
    PreparationFailed(PreparedReleaseError),
    /// The checked stage could not become a new version directory.
    PromotionFailed(PromotionError),
    /// The promoted record could not become the fixed selected machine policy.
    PublicationFailed(PublicationError),
}

impl fmt::Display for InstallCurrentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InstallerInvalid(_) => "the signed installer release is invalid",
            Self::ApplicationAlreadyInstalled => {
                "the application is already installed and requires the update path"
            }
            Self::ExistingPolicyInvalid(_) => {
                "the existing application policy cannot be used safely"
            }
            Self::MachineRootInvalid(_) => "the fixed machine installation root is invalid",
            Self::PreparationFailed(_) => "the signed release could not be prepared",
            Self::PromotionFailed(_) => "the signed release could not be promoted",
            Self::PublicationFailed(_) => "the signed release could not be selected",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for InstallCurrentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstallerInvalid(error) => Some(error),
            Self::ApplicationAlreadyInstalled => None,
            Self::ExistingPolicyInvalid(error) => Some(error),
            Self::MachineRootInvalid(error) => Some(error),
            Self::PreparationFailed(error) => Some(error),
            Self::PromotionFailed(error) => Some(error),
            Self::PublicationFailed(error) => Some(error),
        }
    }
}

/// Installs only the current signed embedded release at its fixed machine root.
///
/// The current executable must first pass the signed embedded-release gate. Its
/// validated identity must have no existing selected machine policy; an update
/// requires the separate strict publisher-and-version gate. The identity then
/// selects the internal Program Files root, after which preparation verifies the
/// current release again before staging. This operation accepts no path,
/// application identity, registry data, executable, certificate, network input,
/// or policy. It does not create trust, launch a process, create application
/// data, or remove an existing release.
pub fn install_current_signed_release() -> Result<InstalledRelease, InstallCurrentError> {
    let release = verify_current_signed_release().map_err(InstallCurrentError::InstallerInvalid)?;
    require_no_selected_policy(release.release().manifest().application_id())?;
    let root = current_machine_application_root(release.release().manifest().application_id())
        .map_err(InstallCurrentError::MachineRootInvalid)?;
    let prepared = prepare_current_signed_release(root.path())
        .map_err(InstallCurrentError::PreparationFailed)?;
    let promoted =
        promote_prepared_release(prepared).map_err(InstallCurrentError::PromotionFailed)?;
    let published =
        publish_promoted_release(promoted).map_err(InstallCurrentError::PublicationFailed)?;
    Ok(InstalledRelease { published })
}

fn require_no_selected_policy(application_id: &str) -> Result<(), InstallCurrentError> {
    initial_policy_state(load_installed_application(application_id).map(|_| ()))
}

fn initial_policy_state(policy: Result<(), PolicyStoreError>) -> Result<(), InstallCurrentError> {
    match policy {
        Ok(()) => Err(InstallCurrentError::ApplicationAlreadyInstalled),
        Err(PolicyStoreError::RecordNotFound) => Ok(()),
        Err(error) => Err(InstallCurrentError::ExistingPolicyInvalid(error)),
    }
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use anodrel_windows_policy::PolicyStoreError;

    use super::{InstallCurrentError, initial_policy_state, install_current_signed_release};
    use crate::SignedReleaseError;

    #[test]
    fn an_unsigned_current_image_stops_before_machine_root_selection() {
        assert!(matches!(
            install_current_signed_release(),
            Err(InstallCurrentError::InstallerInvalid(
                SignedReleaseError::SignatureInvalid(SignatureError::TrustRejected)
            ))
        ));
    }

    #[test]
    fn only_a_missing_machine_record_can_start_an_initial_install() {
        assert!(initial_policy_state(Err(PolicyStoreError::RecordNotFound)).is_ok());
        assert!(matches!(
            initial_policy_state(Ok(())),
            Err(InstallCurrentError::ApplicationAlreadyInstalled)
        ));
        assert!(matches!(
            initial_policy_state(Err(PolicyStoreError::AccessDenied)),
            Err(InstallCurrentError::ExistingPolicyInvalid(
                PolicyStoreError::AccessDenied
            ))
        ));
    }
}
