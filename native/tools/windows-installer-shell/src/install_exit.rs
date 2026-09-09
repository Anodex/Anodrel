//! Fixed, path-free outcomes for the elevated initial-install child.

use anodrel_windows_installer::InstallCurrentError;

const INVALID_RELEASE: u32 = 10;
const ALREADY_INSTALLED: u32 = 11;
const EXISTING_POLICY: u32 = 12;
const MACHINE_ROOT: u32 = 13;
const PREPARATION: u32 = 14;
const PROMOTION: u32 = 15;
const PUBLICATION: u32 = 16;
const PRODUCT_SHORTCUT: u32 = 17;
const APPS_FEATURES: u32 = 18;

/// Returns the fixed conventional exit code for one install transaction error.
pub(super) const fn code_for(error: &InstallCurrentError) -> u8 {
    match error {
        InstallCurrentError::InstallerInvalid(_) => INVALID_RELEASE as u8,
        InstallCurrentError::ApplicationAlreadyInstalled => ALREADY_INSTALLED as u8,
        InstallCurrentError::ExistingPolicyInvalid(_) => EXISTING_POLICY as u8,
        InstallCurrentError::MachineRootInvalid(_) => MACHINE_ROOT as u8,
        InstallCurrentError::PreparationFailed(_) => PREPARATION as u8,
        InstallCurrentError::PromotionFailed(_) => PROMOTION as u8,
        InstallCurrentError::PublicationFailed(_) => PUBLICATION as u8,
        InstallCurrentError::ProductShortcutRegistrationFailed(_) => PRODUCT_SHORTCUT as u8,
        InstallCurrentError::AppsFeaturesRegistrationFailed(_) => APPS_FEATURES as u8,
    }
}

/// Converts one observed child exit into a safe no-argument installer message.
///
/// A message names only a fixed installation stage. It never repeats child
/// output, raw Windows errors, paths, registry data, or whether policy changed.
pub(super) const fn message_for_failed_code(exit_code: u32) -> &'static str {
    match exit_code {
        INVALID_RELEASE => "the signed installer release is invalid",
        ALREADY_INSTALLED => "the application is already installed and requires the update path",
        EXISTING_POLICY => "the existing application policy cannot be used safely",
        MACHINE_ROOT => "the fixed machine installation root is invalid",
        PREPARATION => "the signed release could not be prepared",
        PROMOTION => "the signed release could not be promoted",
        PUBLICATION => "the signed release could not be selected",
        PRODUCT_SHORTCUT => "Start-menu registration did not complete",
        APPS_FEATURES => "Apps & features registration did not complete",
        _ => "the installer reported that installation failed",
    }
}

#[cfg(test)]
mod tests {
    use anodrel_windows_installer::InstallCurrentError;

    use super::{code_for, message_for_failed_code};

    #[test]
    fn known_child_failures_remain_safe_but_distinguishable() {
        assert_eq!(
            message_for_failed_code(17),
            "Start-menu registration did not complete"
        );
        assert_eq!(
            message_for_failed_code(18),
            "Apps & features registration did not complete"
        );
    }

    #[test]
    fn unknown_child_failures_do_not_leak_process_details() {
        assert_eq!(
            message_for_failed_code(255),
            "the installer reported that installation failed"
        );
    }

    #[test]
    fn install_errors_use_only_the_reserved_nonzero_range() {
        assert_eq!(
            code_for(&InstallCurrentError::ApplicationAlreadyInstalled),
            11
        );
    }
}
