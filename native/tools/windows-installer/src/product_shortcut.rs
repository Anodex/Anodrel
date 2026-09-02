//! Signed selected-policy preflight for a later Start-menu shortcut writer.

use std::fmt;

use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{SignedReleaseError, verify_current_signed_release};

/// Opaque proof that the current selected release has signed product metadata.
///
/// This does not retain a target path or product text. A later shell-link
/// operation must establish its own fresh proof immediately before it writes
/// the fixed Windows registration surface.
pub struct VerifiedProductShortcutTarget {
    _private: (),
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
    /// The selected record predates signed product display metadata.
    ProductMetadataUnavailable,
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
            Self::ProductMetadataUnavailable => {
                "the selected application does not declare signed product metadata"
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
            | Self::ProductMetadataUnavailable => None,
        }
    }
}

/// Proves one current selected release is eligible for a fixed Start-menu link.
///
/// The signed current installer chooses the application identity. This reads
/// only that identity's selected machine policy, validates the selected
/// executable's Authenticode signer against both policy and installer, and
/// requires version 1.21 signed product metadata. It does not create or remove
/// a shortcut, query a shell folder, initialize COM, write policy, elevate,
/// launch an application, or expose product data.
pub fn verify_current_product_shortcut_target()
-> Result<VerifiedProductShortcutTarget, ProductShortcutPreflightError> {
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
    if selected.product_metadata().is_none() {
        return Err(ProductShortcutPreflightError::ProductMetadataUnavailable);
    }
    Ok(VerifiedProductShortcutTarget { _private: () })
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{
        ProductShortcutPreflightError, VerifiedProductShortcutTarget,
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
    fn failure_and_debug_text_do_not_disclose_product_or_machine_paths() {
        assert_eq!(
            ProductShortcutPreflightError::ProductMetadataUnavailable.to_string(),
            "the selected application does not declare signed product metadata"
        );
        assert_eq!(
            format!("{:?}", VerifiedProductShortcutTarget { _private: () }),
            "VerifiedProductShortcutTarget(..)"
        );
    }
}
