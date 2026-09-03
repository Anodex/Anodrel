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

/// A completed removal of one fixed all-users Start-menu registration.
pub struct RemovedProductShortcut {
    _private: (),
}

impl fmt::Debug for RegisteredProductShortcut {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RegisteredProductShortcut(..)")
    }
}

impl fmt::Debug for RemovedProductShortcut {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RemovedProductShortcut(..)")
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
    /// The selected record has no verified product launcher.
    ProductLauncherUnavailable,
    /// Windows did not accept the selected launcher signature.
    LauncherSignatureInvalid(SignatureError),
    /// The selected launcher signer differed from its fixed machine policy.
    LauncherPolicyPublisherMismatch,
    /// The selected launcher signer differed from the signed installer.
    InstallerLauncherPublisherMismatch,
    /// A selected identity could not become the fixed launcher command.
    LauncherArgumentsInvalid,
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
            Self::ProductLauncherUnavailable => {
                "the selected application does not declare a product launcher"
            }
            Self::LauncherSignatureInvalid(_) => {
                "Windows did not accept the selected product launcher signature"
            }
            Self::LauncherPolicyPublisherMismatch => {
                "the selected product launcher publisher does not match policy"
            }
            Self::InstallerLauncherPublisherMismatch => {
                "the selected product launcher publisher does not match the installer"
            }
            Self::LauncherArgumentsInvalid => "the selected product launcher command is invalid",
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
            Self::LauncherSignatureInvalid(error) => Some(error),
            Self::SelectedPolicyPublisherMismatch
            | Self::InstallerPublisherMismatch
            | Self::StartMenuNameUnavailable
            | Self::ProductLauncherUnavailable
            | Self::LauncherPolicyPublisherMismatch
            | Self::InstallerLauncherPublisherMismatch
            | Self::LauncherArgumentsInvalid => None,
        }
    }
}

/// A safe failure while writing the fixed all-users Start-menu link.
#[derive(Debug)]
pub enum ProductShortcutRegistrationError {
    /// Fresh selected-policy proof did not establish a shortcut target.
    TargetInvalid(ProductShortcutPreflightError),
    /// Windows could not update the fixed Start-menu link safely.
    ShellOperationFailed,
}

impl fmt::Display for ProductShortcutRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TargetInvalid(_) => "the Start-menu shortcut target is invalid",
            Self::ShellOperationFailed => {
                "the Windows Start-menu shortcut could not be updated safely"
            }
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
    launcher_path: Option<PathBuf>,
    package_root: PathBuf,
    application_id: String,
    start_menu_name: Option<StartMenuName>,
}

/// Previously selected optional product-link data held only during one policy transition.
pub(crate) struct PriorProductShortcut {
    start_menu_name: Option<StartMenuName>,
}

impl PriorProductShortcut {
    pub(crate) const fn none() -> Self {
        Self {
            start_menu_name: None,
        }
    }
}

/// Proves one current selected release is eligible for a fixed Start-menu link.
///
/// The signed current installer chooses the application identity. This reads
/// only that identity's selected machine policy, validates the selected
/// child and launcher Authenticode signers against both policy and installer,
/// and requires version 1.23 signed product-launch metadata and a Start-menu
/// name. It does not create or remove
/// a shortcut, query a shell folder, initialize COM, write policy, elevate,
/// launch an application, or expose product data.
pub fn verify_current_product_shortcut_target()
-> Result<VerifiedProductShortcutTarget, ProductShortcutPreflightError> {
    let target = select_current_product_shortcut_target()?;
    require_start_menu_name(&target)?;
    require_product_launcher(&target)?;
    product_launch_arguments(&target)?;
    Ok(VerifiedProductShortcutTarget { _private: () })
}

/// Replaces the fixed all-users Start-menu link from fresh selected policy.
///
/// The function accepts no application input. It repeats the signed-policy
/// proof immediately before asking Windows to create one link under the common
/// Programs folder. The launcher target, working directory, generated fixed
/// arguments, and signed filename come only from that fresh proof. It does not
/// create an Application User Model ID, accept an argument, launch an
/// application, alter machine policy, or report a person's interaction with the
/// Start menu.
pub fn refresh_current_product_shortcut()
-> Result<RegisteredProductShortcut, ProductShortcutRegistrationError> {
    let target = select_current_product_shortcut_target()
        .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
    let start_menu_name = require_start_menu_name(&target)
        .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
    let launcher_path = require_product_launcher(&target)
        .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
    let arguments = product_launch_arguments(&target)
        .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
    raw::replace_common_programs_shortcut(
        launcher_path,
        &target.package_root,
        &arguments,
        start_menu_name,
    )
    .map_err(|_| ProductShortcutRegistrationError::ShellOperationFailed)?;
    Ok(RegisteredProductShortcut { _private: () })
}

/// Captures only the current optional signed link name before a policy change.
///
/// This repeats selected-policy and signer proof, but does not access Windows
/// shell folders or change any registration. A record without Start-menu
/// metadata is valid and produces an empty prior state.
pub(crate) fn capture_current_product_shortcut()
-> Result<PriorProductShortcut, ProductShortcutPreflightError> {
    let target = select_current_product_shortcut_target()?;
    Ok(PriorProductShortcut {
        start_menu_name: target.start_menu_name,
    })
}

/// Synchronizes the fixed product link after machine policy selected a release.
///
/// The prior value comes only from `capture_current_product_shortcut` before
/// the policy transition. Fresh selected-policy proof chooses the replacement.
/// A new link is persisted before a differently named older link is removed.
pub(crate) fn synchronize_current_product_shortcut(
    prior: PriorProductShortcut,
) -> Result<RegisteredProductShortcut, ProductShortcutRegistrationError> {
    let target = select_current_product_shortcut_target()
        .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
    let active_start_menu_name = if let Some(launcher_path) = &target.launcher_path {
        let start_menu_name = require_start_menu_name(&target)
            .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
        let arguments = product_launch_arguments(&target)
            .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
        raw::replace_common_programs_shortcut(
            launcher_path,
            &target.package_root,
            &arguments,
            start_menu_name,
        )
        .map_err(|_| ProductShortcutRegistrationError::ShellOperationFailed)?;
        Some(start_menu_name)
    } else {
        None
    };
    if let Some(start_menu_name) =
        stale_start_menu_name(prior.start_menu_name.as_ref(), active_start_menu_name)
    {
        raw::remove_common_programs_shortcut(start_menu_name)
            .map_err(|_| ProductShortcutRegistrationError::ShellOperationFailed)?;
    }
    Ok(RegisteredProductShortcut { _private: () })
}

/// Removes the current selected product's fixed link before an uninstall.
///
/// Fresh selected-policy proof chooses the only filename that may be removed.
/// A legacy selected record with no signed Start-menu name has no link to
/// remove. This does not alter policy or package files.
pub fn remove_current_product_shortcut()
-> Result<RemovedProductShortcut, ProductShortcutRegistrationError> {
    let target = select_current_product_shortcut_target()
        .map_err(ProductShortcutRegistrationError::TargetInvalid)?;
    if let Some(start_menu_name) = &target.start_menu_name {
        raw::remove_common_programs_shortcut(start_menu_name)
            .map_err(|_| ProductShortcutRegistrationError::ShellOperationFailed)?;
    }
    Ok(RemovedProductShortcut { _private: () })
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
    let launcher_path = selected.product_launcher_path().map(PathBuf::from);
    if let Some(launcher_path) = &launcher_path {
        let signer = verify_embedded_signature(launcher_path)
            .map_err(ProductShortcutPreflightError::LauncherSignatureInvalid)?;
        if !selected.matches_publisher(signer.as_bytes()) {
            return Err(ProductShortcutPreflightError::LauncherPolicyPublisherMismatch);
        }
        if !manifest.matches_publisher_fingerprint(signer.as_bytes()) {
            return Err(ProductShortcutPreflightError::InstallerLauncherPublisherMismatch);
        }
    }
    Ok(SelectedProductShortcut {
        launcher_path,
        package_root: selected.package_root().to_path_buf(),
        application_id: selected.identity().application_id().to_owned(),
        start_menu_name: selected.start_menu_name().cloned(),
    })
}

fn require_start_menu_name(
    target: &SelectedProductShortcut,
) -> Result<&StartMenuName, ProductShortcutPreflightError> {
    target
        .start_menu_name
        .as_ref()
        .ok_or(ProductShortcutPreflightError::StartMenuNameUnavailable)
}

fn require_product_launcher(
    target: &SelectedProductShortcut,
) -> Result<&PathBuf, ProductShortcutPreflightError> {
    target
        .launcher_path
        .as_ref()
        .ok_or(ProductShortcutPreflightError::ProductLauncherUnavailable)
}

fn product_launch_arguments(
    target: &SelectedProductShortcut,
) -> Result<raw::ProductLaunchArguments, ProductShortcutPreflightError> {
    raw::ProductLaunchArguments::for_application(&target.application_id)
        .map_err(|_| ProductShortcutPreflightError::LauncherArgumentsInvalid)
}

fn stale_start_menu_name<'a>(
    prior: Option<&'a StartMenuName>,
    current: Option<&StartMenuName>,
) -> Option<&'a StartMenuName> {
    (prior != current).then_some(prior).flatten()
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{
        ProductShortcutPreflightError, ProductShortcutRegistrationError, RegisteredProductShortcut,
        RemovedProductShortcut, VerifiedProductShortcutTarget, refresh_current_product_shortcut,
        stale_start_menu_name, verify_current_product_shortcut_target,
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
        assert_eq!(
            format!("{:?}", RemovedProductShortcut { _private: () }),
            "RemovedProductShortcut(..)"
        );
    }

    #[test]
    fn only_a_differing_prior_signed_name_requires_stale_link_removal() {
        let first = anodrel_application::StartMenuName::new("Anodrel First")
            .expect("safe fixed product name");
        let second = anodrel_application::StartMenuName::new("Anodrel Second")
            .expect("safe fixed product name");
        assert_eq!(stale_start_menu_name(Some(&first), Some(&first)), None);
        assert_eq!(
            stale_start_menu_name(Some(&first), Some(&second)),
            Some(&first)
        );
        assert_eq!(stale_start_menu_name(Some(&first), None), Some(&first));
        assert_eq!(stale_start_menu_name(None, Some(&second)), None);
    }
}
