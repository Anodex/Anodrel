//! Signed policy-backed rollback preflight and fixed record restoration.

use std::{fmt, path::Path};

use anodrel_windows_policy::{
    PolicyStoreError, load_installed_application, load_previous_installed_application,
};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::machine_root::existing_machine_application_root;
use crate::product_shortcut::{
    capture_current_product_shortcut, synchronize_current_product_shortcut,
};
use crate::publication::restore_previous_record;
use crate::{
    MachineRootError, PackageVersion, ProductShortcutPreflightError,
    ProductShortcutRegistrationError, RollbackPublicationError, SignedReleaseError,
    verify_current_signed_release,
};

/// A current signed release and retained policy record eligible for fixed rollback.
pub struct VerifiedRollbackTarget {
    application_id: String,
}

impl VerifiedRollbackTarget {
    #[must_use]
    pub(crate) fn application_id(&self) -> &str {
        &self.application_id
    }
}

impl fmt::Debug for VerifiedRollbackTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedRollbackTarget")
            .field("application_id", &self.application_id)
            .finish_non_exhaustive()
    }
}

/// A prior release selected by a completed fixed rollback transaction.
pub struct RolledBackRelease {
    target: VerifiedRollbackTarget,
}

impl fmt::Debug for RolledBackRelease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RolledBackRelease")
            .field(&self.target)
            .finish()
    }
}

/// The current signed installer could not establish a safe rollback target.
#[derive(Debug)]
pub enum RollbackPreflightError {
    /// The current installer did not pass its signed embedded-release gate.
    InstallerInvalid(SignedReleaseError),
    /// The fixed existing machine root was unavailable or unsafe.
    MachineRootInvalid(MachineRootError),
    /// The current fixed application policy was missing or invalid.
    CurrentPolicyInvalid(PolicyStoreError),
    /// The retained fixed prior policy was missing or invalid.
    PreviousPolicyInvalid(PolicyStoreError),
    /// Windows did not accept the current executable signature.
    CurrentSignatureInvalid(SignatureError),
    /// Windows did not accept the retained prior executable signature.
    PreviousSignatureInvalid(SignatureError),
    /// The current executable signer differed from its current policy record.
    CurrentPolicyPublisherMismatch,
    /// The current executable signer differed from the installer publisher.
    CurrentInstallerPublisherMismatch,
    /// The prior executable signer differed from its retained policy record.
    PreviousPolicyPublisherMismatch,
    /// The prior executable signer differed from the installer publisher.
    PreviousInstallerPublisherMismatch,
    /// A policy package root was not a direct version child of the owned root.
    PackageRootOutsideOwnedRoot,
    /// The retained version was equal to or newer than the current version.
    PreviousVersionNotOlder,
}

impl fmt::Display for RollbackPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InstallerInvalid(_) => "the signed installer release is invalid",
            Self::MachineRootInvalid(_) => "the fixed machine installation root is invalid",
            Self::CurrentPolicyInvalid(_) => "the current application policy is invalid",
            Self::PreviousPolicyInvalid(_) => "the retained prior policy is invalid",
            Self::CurrentSignatureInvalid(_) => {
                "Windows did not accept the current executable signature"
            }
            Self::PreviousSignatureInvalid(_) => {
                "Windows did not accept the retained prior executable signature"
            }
            Self::CurrentPolicyPublisherMismatch => {
                "the current executable publisher does not match policy"
            }
            Self::CurrentInstallerPublisherMismatch => {
                "the current executable publisher does not match the installer"
            }
            Self::PreviousPolicyPublisherMismatch => {
                "the retained prior executable publisher does not match policy"
            }
            Self::PreviousInstallerPublisherMismatch => {
                "the retained prior executable publisher does not match the installer"
            }
            Self::PackageRootOutsideOwnedRoot => {
                "a rollback package root is outside the owned version hierarchy"
            }
            Self::PreviousVersionNotOlder => {
                "the retained prior release is not older than the selected release"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RollbackPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstallerInvalid(error) => Some(error),
            Self::MachineRootInvalid(error) => Some(error),
            Self::CurrentPolicyInvalid(error) => Some(error),
            Self::PreviousPolicyInvalid(error) => Some(error),
            Self::CurrentSignatureInvalid(error) => Some(error),
            Self::PreviousSignatureInvalid(error) => Some(error),
            Self::CurrentPolicyPublisherMismatch
            | Self::CurrentInstallerPublisherMismatch
            | Self::PreviousPolicyPublisherMismatch
            | Self::PreviousInstallerPublisherMismatch
            | Self::PackageRootOutsideOwnedRoot
            | Self::PreviousVersionNotOlder => None,
        }
    }
}

/// A fixed signed rollback transaction could not complete.
#[derive(Debug)]
pub enum RollbackCurrentError {
    /// The current signed installer did not establish a safe rollback target.
    PreflightInvalid(RollbackPreflightError),
    /// The retained prior record could not become the fixed selected record.
    PublicationFailed(RollbackPublicationError),
    /// The current product link could not be proved before policy changed.
    PriorProductShortcutInvalid(ProductShortcutPreflightError),
    /// Policy selected the prior release, but its fixed Start-menu link is incomplete.
    ProductShortcutRegistrationFailed(ProductShortcutRegistrationError),
}

impl fmt::Display for RollbackCurrentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreflightInvalid(_) => formatter.write_str("the rollback target is invalid"),
            Self::PublicationFailed(_) => {
                formatter.write_str("the rollback policy could not be selected")
            }
            Self::PriorProductShortcutInvalid(_) => {
                formatter.write_str("the current Start-menu registration state is invalid")
            }
            Self::ProductShortcutRegistrationFailed(_) => formatter
                .write_str("the rollback was selected but Start-menu registration is incomplete"),
        }
    }
}

impl std::error::Error for RollbackCurrentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PreflightInvalid(error) => Some(error),
            Self::PublicationFailed(error) => Some(error),
            Self::PriorProductShortcutInvalid(error) => Some(error),
            Self::ProductShortcutRegistrationFailed(error) => Some(error),
        }
    }
}

/// Verifies the only retained policy record the current signed release may restore.
pub fn verify_current_rollback_target() -> Result<VerifiedRollbackTarget, RollbackPreflightError> {
    let release =
        verify_current_signed_release().map_err(RollbackPreflightError::InstallerInvalid)?;
    let manifest = release.release().manifest();
    let root = existing_machine_application_root(manifest.application_id())
        .map_err(RollbackPreflightError::MachineRootInvalid)?;
    let current = load_installed_application(manifest.application_id())
        .map_err(RollbackPreflightError::CurrentPolicyInvalid)?;
    let previous = load_previous_installed_application(manifest.application_id())
        .map_err(RollbackPreflightError::PreviousPolicyInvalid)?;
    let current_signer = verify_embedded_signature(current.executable_path())
        .map_err(RollbackPreflightError::CurrentSignatureInvalid)?;
    if !current.matches_publisher(current_signer.as_bytes()) {
        return Err(RollbackPreflightError::CurrentPolicyPublisherMismatch);
    }
    if !manifest.matches_publisher_fingerprint(current_signer.as_bytes()) {
        return Err(RollbackPreflightError::CurrentInstallerPublisherMismatch);
    }
    let previous_signer = verify_embedded_signature(previous.executable_path())
        .map_err(RollbackPreflightError::PreviousSignatureInvalid)?;
    if !previous.matches_publisher(previous_signer.as_bytes()) {
        return Err(RollbackPreflightError::PreviousPolicyPublisherMismatch);
    }
    if !manifest.matches_publisher_fingerprint(previous_signer.as_bytes()) {
        return Err(RollbackPreflightError::PreviousInstallerPublisherMismatch);
    }
    let current_version = package_version_under_root(root.path(), current.package_root())
        .ok_or(RollbackPreflightError::PackageRootOutsideOwnedRoot)?;
    let previous_version = package_version_under_root(root.path(), previous.package_root())
        .ok_or(RollbackPreflightError::PackageRootOutsideOwnedRoot)?;
    if previous_version >= current_version {
        return Err(RollbackPreflightError::PreviousVersionNotOlder);
    }
    Ok(VerifiedRollbackTarget {
        application_id: manifest.application_id().to_owned(),
    })
}

/// Restores the fixed retained prior policy record after complete rollback preflight.
pub fn rollback_current_signed_release() -> Result<RolledBackRelease, RollbackCurrentError> {
    let target =
        verify_current_rollback_target().map_err(RollbackCurrentError::PreflightInvalid)?;
    let prior = capture_current_product_shortcut()
        .map_err(RollbackCurrentError::PriorProductShortcutInvalid)?;
    restore_previous_record(target.application_id())
        .map_err(RollbackCurrentError::PublicationFailed)?;
    synchronize_current_product_shortcut(prior)
        .map_err(RollbackCurrentError::ProductShortcutRegistrationFailed)?;
    Ok(RolledBackRelease { target })
}

fn package_version_under_root(root: &Path, package_root: &Path) -> Option<PackageVersion> {
    (package_root.parent()? == root)
        .then(|| package_root.file_name()?.to_str())?
        .and_then(PackageVersion::from_canonical_directory_name)
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{
        RollbackPreflightError, package_version_under_root, verify_current_rollback_target,
    };
    use crate::SignedReleaseError;

    #[test]
    fn direct_owned_version_children_are_the_only_rollback_version_sources() {
        let root =
            std::path::Path::new(r"C:\\Program Files\\Anodrel\\Applications\\org.anodrel.test");
        let version = package_version_under_root(root, &root.join("1.2.3"))
            .expect("a direct canonical version child parses");
        assert_eq!(
            (version.major(), version.minor(), version.patch()),
            (1, 2, 3)
        );
        assert!(package_version_under_root(root, &root.join("nested").join("1.2.3")).is_none());
        assert!(package_version_under_root(root, &root.join("1.02.3")).is_none());
    }

    #[test]
    fn an_unsigned_current_installer_cannot_select_a_rollback_target() {
        assert!(matches!(
            verify_current_rollback_target(),
            Err(RollbackPreflightError::InstallerInvalid(
                SignedReleaseError::SignatureInvalid(SignatureError::TrustRejected)
            ))
        ));
    }
}
