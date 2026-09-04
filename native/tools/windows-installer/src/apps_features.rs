//! Selected-policy proof for one fixed Windows Apps & features entry.

use std::{fmt, path::PathBuf};

use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{
    InstallerImageError, PackageVersion, SignedReleaseError,
    installed_uninstaller::installed_uninstaller_path, verify_current_signed_release,
    verify_locked_installer_image,
};

mod raw;

/// Opaque proof for one current selected Apps & features registration.
pub struct VerifiedAppsFeaturesTarget {
    application_id: String,
    display_name: String,
    publisher_name: String,
    version: PackageVersion,
    uninstaller_path: PathBuf,
}

/// A completed fixed Apps & features registration.
pub struct RegisteredAppsFeatures {
    _private: (),
}

/// A completed removal of one fixed Apps & features registration.
pub struct RemovedAppsFeatures {
    _private: (),
}

impl fmt::Debug for VerifiedAppsFeaturesTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedAppsFeaturesTarget(..)")
    }
}

impl fmt::Debug for RegisteredAppsFeatures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RegisteredAppsFeatures(..)")
    }
}

impl fmt::Debug for RemovedAppsFeatures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RemovedAppsFeatures(..)")
    }
}

/// A selected release could not prove its fixed Apps & features entry.
#[derive(Debug)]
pub enum AppsFeaturesPreflightError {
    /// The current installer did not pass its signed embedded-release gate.
    CurrentInstallerInvalid(SignedReleaseError),
    /// The selected machine record was unavailable or invalid.
    SelectedPolicyInvalid(PolicyStoreError),
    /// Windows did not accept the selected application executable signature.
    SelectedSignatureInvalid(SignatureError),
    /// The selected application signer differed from selected policy.
    SelectedPolicyPublisherMismatch,
    /// The current installer release publisher differed from the selected application.
    CurrentInstallerPublisherMismatch,
    /// The selected release did not carry signed product display metadata.
    ProductMetadataUnavailable,
    /// The selected package directory did not express one canonical version.
    SelectedVersionInvalid,
    /// The fixed installed uninstaller image did not pass its locked release gate.
    InstalledUninstallerInvalid(InstallerImageError),
    /// The installed uninstaller identity differed from the selected policy.
    UninstallerIdentityMismatch,
    /// The installed uninstaller version differed from the selected package root.
    UninstallerVersionMismatch,
    /// The installed uninstaller publisher differed from the selected policy.
    UninstallerPublisherMismatch,
    /// The installed uninstaller product display text differed from selected policy.
    UninstallerProductMismatch,
}

impl fmt::Display for AppsFeaturesPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CurrentInstallerInvalid(_) => "the signed installer release is invalid",
            Self::SelectedPolicyInvalid(_) => "the selected application policy is invalid",
            Self::SelectedSignatureInvalid(_) => {
                "Windows did not accept the selected executable signature"
            }
            Self::SelectedPolicyPublisherMismatch => {
                "the selected executable publisher does not match policy"
            }
            Self::CurrentInstallerPublisherMismatch => {
                "the selected application publisher does not match the installer"
            }
            Self::ProductMetadataUnavailable => {
                "the selected application has no signed product metadata"
            }
            Self::SelectedVersionInvalid => "the selected application version is invalid",
            Self::InstalledUninstallerInvalid(_) => "the installed uninstaller image is invalid",
            Self::UninstallerIdentityMismatch => {
                "the installed uninstaller does not match the selected application"
            }
            Self::UninstallerVersionMismatch => {
                "the installed uninstaller version does not match the selected application"
            }
            Self::UninstallerPublisherMismatch => {
                "the installed uninstaller publisher does not match the selected application"
            }
            Self::UninstallerProductMismatch => {
                "the installed uninstaller product metadata does not match policy"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AppsFeaturesPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CurrentInstallerInvalid(error) => Some(error),
            Self::SelectedPolicyInvalid(error) => Some(error),
            Self::SelectedSignatureInvalid(error) => Some(error),
            Self::InstalledUninstallerInvalid(error) => Some(error),
            Self::SelectedPolicyPublisherMismatch
            | Self::CurrentInstallerPublisherMismatch
            | Self::ProductMetadataUnavailable
            | Self::SelectedVersionInvalid
            | Self::UninstallerIdentityMismatch
            | Self::UninstallerVersionMismatch
            | Self::UninstallerPublisherMismatch
            | Self::UninstallerProductMismatch => None,
        }
    }
}

/// A fixed Apps & features registry operation could not complete safely.
#[derive(Debug)]
pub enum AppsFeaturesRegistrationError {
    /// Fresh selected-policy proof could not establish the one fixed target.
    TargetInvalid(AppsFeaturesPreflightError),
    /// Windows could not write or remove the one fixed registry entry.
    RegistryOperationFailed,
}

impl fmt::Display for AppsFeaturesRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TargetInvalid(_) => "the Apps & features registration target is invalid",
            Self::RegistryOperationFailed => {
                "the Apps & features registration could not be updated safely"
            }
        })
    }
}

impl std::error::Error for AppsFeaturesRegistrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TargetInvalid(error) => Some(error),
            Self::RegistryOperationFailed => None,
        }
    }
}

/// Proves the fixed selected release data required for Apps & features.
///
/// This obtains no registry write handle and does not create registration,
/// modify policy, launch a process, elevate, or expose product data to an
/// application protocol surface.
pub fn verify_current_apps_features_target()
-> Result<VerifiedAppsFeaturesTarget, AppsFeaturesPreflightError> {
    let current = verify_current_signed_release()
        .map_err(AppsFeaturesPreflightError::CurrentInstallerInvalid)?;
    let manifest = current.release().manifest();
    let selected = load_installed_application(manifest.application_id())
        .map_err(AppsFeaturesPreflightError::SelectedPolicyInvalid)?;
    let selected_signer = verify_embedded_signature(selected.executable_path())
        .map_err(AppsFeaturesPreflightError::SelectedSignatureInvalid)?;
    if !selected.matches_publisher(selected_signer.as_bytes()) {
        return Err(AppsFeaturesPreflightError::SelectedPolicyPublisherMismatch);
    }
    if !manifest.matches_publisher_fingerprint(selected_signer.as_bytes()) {
        return Err(AppsFeaturesPreflightError::CurrentInstallerPublisherMismatch);
    }
    let product = selected
        .product_metadata()
        .ok_or(AppsFeaturesPreflightError::ProductMetadataUnavailable)?;
    let version = selected_version(selected.package_root())
        .ok_or(AppsFeaturesPreflightError::SelectedVersionInvalid)?;
    let uninstaller_path = installed_uninstaller_path(selected.package_root());
    let uninstaller = verify_locked_installer_image(&uninstaller_path)
        .map_err(AppsFeaturesPreflightError::InstalledUninstallerInvalid)?;
    let uninstaller_manifest = uninstaller.manifest();
    if uninstaller_manifest.application_id() != selected.identity().application_id() {
        return Err(AppsFeaturesPreflightError::UninstallerIdentityMismatch);
    }
    if uninstaller_manifest.package_version() != version {
        return Err(AppsFeaturesPreflightError::UninstallerVersionMismatch);
    }
    let uninstaller_signer = verify_embedded_signature(&uninstaller_path).map_err(|error| {
        AppsFeaturesPreflightError::InstalledUninstallerInvalid(
            InstallerImageError::SignatureInvalid(error),
        )
    })?;
    if !selected.matches_publisher(uninstaller_signer.as_bytes()) {
        return Err(AppsFeaturesPreflightError::UninstallerPublisherMismatch);
    }
    if uninstaller_manifest.product_metadata() != Some(product) {
        return Err(AppsFeaturesPreflightError::UninstallerProductMismatch);
    }
    Ok(VerifiedAppsFeaturesTarget {
        application_id: selected.identity().application_id().to_owned(),
        display_name: product.display_name().to_owned(),
        publisher_name: product.publisher_name().to_owned(),
        version,
        uninstaller_path,
    })
}

/// Replaces the one fixed Apps & features entry from fresh selected policy.
pub fn refresh_current_apps_features()
-> Result<RegisteredAppsFeatures, AppsFeaturesRegistrationError> {
    let target = verify_current_apps_features_target()
        .map_err(AppsFeaturesRegistrationError::TargetInvalid)?;
    raw::write(&target).map_err(|_| AppsFeaturesRegistrationError::RegistryOperationFailed)?;
    Ok(RegisteredAppsFeatures { _private: () })
}

/// Removes the one fixed Apps & features entry from fresh selected policy.
///
/// A missing entry is harmless. The caller controls no key, application, path,
/// or registry value, and this function does not alter policy or package files.
pub fn remove_current_apps_features() -> Result<RemovedAppsFeatures, AppsFeaturesRegistrationError>
{
    let target = verify_current_apps_features_target()
        .map_err(AppsFeaturesRegistrationError::TargetInvalid)?;
    raw::remove(&target).map_err(|_| AppsFeaturesRegistrationError::RegistryOperationFailed)?;
    Ok(RemovedAppsFeatures { _private: () })
}

fn selected_version(package_root: &std::path::Path) -> Option<PackageVersion> {
    package_root
        .file_name()?
        .to_str()
        .and_then(PackageVersion::from_canonical_directory_name)
}

#[cfg(test)]
mod tests {
    use super::{
        AppsFeaturesPreflightError, RegisteredAppsFeatures, RemovedAppsFeatures,
        VerifiedAppsFeaturesTarget, selected_version,
    };

    #[test]
    fn only_canonical_selected_version_names_can_reach_product_registration() {
        assert_eq!(
            selected_version(std::path::Path::new("C:\\Program Files\\Anodrel\\1.2.3")),
            Some(crate::PackageVersion::new(1, 2, 3))
        );
        assert!(selected_version(std::path::Path::new("C:\\Anodrel\\1.2.03")).is_none());
    }

    #[test]
    fn product_registration_debug_and_errors_keep_paths_private() {
        assert_eq!(
            format!("{:?}", RegisteredAppsFeatures { _private: () }),
            "RegisteredAppsFeatures(..)"
        );
        assert_eq!(
            format!("{:?}", RemovedAppsFeatures { _private: () }),
            "RemovedAppsFeatures(..)"
        );
        assert_eq!(
            format!(
                "{:?}",
                VerifiedAppsFeaturesTarget {
                    application_id: "org.anodrel.test".to_owned(),
                    display_name: "Test".to_owned(),
                    publisher_name: "Anodrel".to_owned(),
                    version: crate::PackageVersion::new(1, 2, 3),
                    uninstaller_path: std::path::PathBuf::from("C:\\hidden.exe"),
                }
            ),
            "VerifiedAppsFeaturesTarget(..)"
        );
        assert_eq!(
            AppsFeaturesPreflightError::SelectedVersionInvalid.to_string(),
            "the selected application version is invalid"
        );
    }
}
