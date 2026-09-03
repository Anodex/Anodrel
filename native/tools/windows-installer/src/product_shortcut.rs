//! Signed selected-policy registration for one fixed Windows Start-menu link.

use std::{fmt, path::PathBuf};

use anodrel_application::StartMenuName;
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{SignedReleaseError, verify_current_signed_release};

mod raw;

/// Opaque proof that the current selected release has signed Start-menu data.
///
/// This does not retain a target path or product text. A later shell-link
/// operation must establish its own fresh proof immediately before it writes
/// the fixed Windows registration surface.
pub struct VerifiedProductShortcutTarget {
    _private: (),
}

/// A completed fixed all-users Start-menu registration.
pub struct RegisteredProductShortcut {
    _private: (),
}

impl fmt::Debug for RegisteredProductShortcut {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RegisteredProductShortcut(..)")
    }
}

impl fmt::Debug for VerifiedProductShortcutTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedProductShortcutTarget(..)")
    }
}

/// A safe failure while proving the selected release for a Start-menu surface.
#[derive(Debug)]
pub enum ProductShortcutPreflightError {
    /// The current embedded installer release did not pass its signed gate.
    InstallerInvalid(SignedReleaseError),
    /// The selected machine policy could not be loaded and validated.
    SelectedPolicyInvalid(PolicyStoreError),
    /// Windows did not accept the selected executable's embedded signature.
    SelectedSignatureInvalid(SignatureError),
    /// The selected executable signer differed from its fixed machine policy.
    SelectedPolicyPublisherMismatch,
    /// The selected executable signer differed from the signed installer.
    InstallerPublisherMismatch,
    /// The selected record predates signed Start-menu registration metadata.
    StartMenuNameUnavailable,
}

impl fmt::Display for ProductShortcutPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InstallerInvalid(_) => "the signed installer release is invalid",
            Self::SelectedPolicyInvalid(_) => "the selected application policy is invalid",
            Self::SelectedSignatureInvalid(_) => {
                "Windows did not accept the selected executable signature"
            }
            Self::SelectedPolicyPublisherMismatch => {
                "the selected executable publisher does not match policy"
            }
            Self::InstallerPublisherMismatch => {
                "the selected executable publisher does not match the installer"
            }
            Self::StartMenuNameUnavailable => {
                "the selected application does not declare a signed Start-menu name"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProductShortcutPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstallerInvalid(error) => Some(error),
            Self::SelectedPolicyInvalid(error) => Some(error),
            Self::SelectedSignatureInvalid(error) => Some(error),
            Self::SelectedPolicyPublisherMismatch
            | Self::InstallerPublisherMismatch
            | Self::StartMenuNameUnavailable => None,
        }
    }
}

/// A safe failure while writing the fixed all-users Start-menu link.
#[derive(Debug)]
pub enum ProductShortcutRegistrationError {
    /// Fresh selected-policy proof did not establish a shortcut target.
    TargetInvalid(ProductShortcutPreflightError),
    /// Windows could not create the fixed Start-menu link safely.
    ShellOperationFailed,
}

impl fmt::Display for ProductShortcutRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TargetInvalid(_) => "the Start-menu shortcut target is invalid",
            Self::ShellOperationFailed => "the Windows Start-menu shortcut could not be created",
        })
    }
}

impl std::error::Error for ProductShortcutRegistrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TargetInvalid(error) => Some(error),
            Self::ShellOperationFailed => None,
        }
    }
}

struct SelectedProductShortcut {
    executable_path: PathBuf,
    package_root: PathBuf,
    start_menu_name: StartMenuName,
}

/// Proves one current selected release is eligible for a fixed Start-menu link.
///
/// The signed current installer chooses the application identity. This reads
/// only that identity's selected machine policy, validates the selected
/// executable's Authenticode signer against both policy and installer, and
/// requires version 1.22 signed product metadata and a Start-menu name. It does not create or remove
/// a shortcut, query a shell folder, initialize COM, write policy, elevate,
/// launch an application, or expose product data.
pub fn verify_current_product_shortcut_target()
-> Result<VerifiedProductShortcutTarget, ProductShortcutPreflightError> {
    select_current_product_shortcut_target().map(|_| VerifiedProductShortcutTarget { _private: () })
}

/// Replaces the fixed all-users Start-menu link from fresh selected policy.
///
/// The function accepts no application input. It repeats the signed-policy
/// proof immediately before asking Windows to create one link under the common
/// Programs folder. The target, working directory, and signed filename come
/// only from that fresh proof. It does not create an Application User Model ID,
/// pass an argument, launch an application, alter machine policy, or report a
/// person's interaction with the Start menu.
pub fn refresh_current_product_shortcut()
-> Result<RegisteredProductShortcut, ProductShortcutRegistrationError> {
    let target = select_current_product_shortcut_target()
        .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
    raw::replace_common_programs_shortcut(
        &target.executable_path,
        &target.package_root,
        &target.start_menu_name,
    )
    .map_err(|_| ProductShortcutRegistrationError::ShellOperationFailed)?;
    Ok(RegisteredProductShortcut { _private: () })
}

fn select_current_product_shortcut_target()
-> Result<SelectedProductShortcut, ProductShortcutPreflightError> {
    let release =
        verify_current_signed_release().map_err(ProductShortcutPreflightError::InstallerInvalid)?;
    let manifest = release.release().manifest();
    let selected = load_installed_application(manifest.application_id())
        .map_err(ProductShortcutPreflightError::SelectedPolicyInvalid)?;
    let signer = verify_embedded_signature(selected.executable_path())
        .map_err(ProductShortcutPreflightError::SelectedSignatureInvalid)?;
    if !selected.matches_publisher(signer.as_bytes()) {
        return Err(ProductShortcutPreflightError::SelectedPolicyPublisherMismatch);
    }
    if !manifest.matches_publisher_fingerprint(signer.as_bytes()) {
        return Err(ProductShortcutPreflightError::InstallerPublisherMismatch);
    }
    if selected.product_metadata().is_none() || selected.start_menu_name().is_none() {
        return Err(ProductShortcutPreflightError::StartMenuNameUnavailable);
    }
    Ok(SelectedProductShortcut {
        executable_path: selected.executable_path().to_path_buf(),
        package_root: selected.package_root().to_path_buf(),
        start_menu_name: selected
            .start_menu_name()
            .expect("the preceding selected-policy check requires this value")
            .clone(),
    })
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{
        ProductShortcutPreflightError, ProductShortcutRegistrationError, RegisteredProductShortcut,
        VerifiedProductShortcutTarget, refresh_current_product_shortcut,
        verify_current_product_shortcut_target,
    };
    use crate::SignedReleaseError;

    #[test]
    fn an_unsigned_current_installer_cannot_select_a_product_shortcut_target() {
        assert!(matches!(
            verify_current_product_shortcut_target(),
            Err(ProductShortcutPreflightError::InstallerInvalid(
                SignedReleaseError::SignatureInvalid(SignatureError::TrustRejected)
            ))
        ));
    }

    #[test]
    fn an_unsigned_current_installer_cannot_write_a_product_shortcut() {
        assert!(matches!(
            refresh_current_product_shortcut(),
            Err(ProductShortcutRegistrationError::TargetInvalid(
                ProductShortcutPreflightError::InstallerInvalid(
                    SignedReleaseError::SignatureInvalid(SignatureError::TrustRejected)
                )
            ))
        ));
    }

    #[test]
    fn failure_and_debug_text_do_not_disclose_product_or_machine_paths() {
        assert_eq!(
            ProductShortcutPreflightError::StartMenuNameUnavailable.to_string(),
            "the selected application does not declare a signed Start-menu name"
        );
        assert_eq!(
            format!("{:?}", VerifiedProductShortcutTarget { _private: () }),
            "VerifiedProductShortcutTarget(..)"
        );
        assert_eq!(
            format!("{:?}", RegisteredProductShortcut { _private: () }),
            "RegisteredProductShortcut(..)"
        );
    }
}
