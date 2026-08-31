//! Signed identity preflight for a later owned uninstall transaction.

use std::{fmt, path::PathBuf};

use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{SignedReleaseError, verify_current_signed_release};

/// A signed, policy-selected application eligible for later uninstall work.
pub struct VerifiedUninstallTarget {
    package_root: PathBuf,
}

impl fmt::Debug for VerifiedUninstallTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedUninstallTarget")
            .field(
                "package_root_units",
                &self
                    .package_root
                    .as_os_str()
                    .to_string_lossy()
                    .encode_utf16()
                    .count(),
            )
            .finish_non_exhaustive()
    }
}

/// The current installer could not establish a safe uninstall target.
#[derive(Debug)]
pub enum UninstallPreflightError {
    /// The current installer did not pass its signed embedded-release gate.
    InstallerInvalid(SignedReleaseError),
    /// The fixed machine record was missing or invalid.
    InstalledPolicyInvalid(PolicyStoreError),
    /// Windows did not accept the installed executable signature.
    InstalledSignatureInvalid(SignatureError),
    /// The installed signer differed from its own validated machine record.
    InstalledPublisherMismatch,
    /// The installed signer differed from the signed uninstaller publisher.
    InstallerPublisherMismatch,
}

impl fmt::Display for UninstallPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InstallerInvalid(_) => "the signed installer release is invalid",
            Self::InstalledPolicyInvalid(_) => "the installed application policy is invalid",
            Self::InstalledSignatureInvalid(_) => {
                "Windows did not accept the installed executable signature"
            }
            Self::InstalledPublisherMismatch => {
                "the installed executable publisher does not match policy"
            }
            Self::InstallerPublisherMismatch => {
                "the installer publisher does not match the installed application"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UninstallPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstallerInvalid(error) => Some(error),
            Self::InstalledPolicyInvalid(error) => Some(error),
            Self::InstalledSignatureInvalid(error) => Some(error),
            Self::InstalledPublisherMismatch | Self::InstallerPublisherMismatch => None,
        }
    }
}

/// Verifies the only application the current signed installer may uninstall.
///
/// This does not remove policy, files, data, credentials, or a process. It
/// returns no path or publisher identity through public formatting.
pub fn verify_current_uninstall_target() -> Result<VerifiedUninstallTarget, UninstallPreflightError>
{
    let release =
        verify_current_signed_release().map_err(UninstallPreflightError::InstallerInvalid)?;
    let manifest = release.release().manifest();
    let installed = load_installed_application(manifest.application_id())
        .map_err(UninstallPreflightError::InstalledPolicyInvalid)?;
    let signer = verify_embedded_signature(installed.executable_path())
        .map_err(UninstallPreflightError::InstalledSignatureInvalid)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(UninstallPreflightError::InstalledPublisherMismatch);
    }
    if !manifest.matches_publisher_fingerprint(signer.as_bytes()) {
        return Err(UninstallPreflightError::InstallerPublisherMismatch);
    }
    Ok(VerifiedUninstallTarget {
        package_root: installed.package_root().to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{UninstallPreflightError, verify_current_uninstall_target};
    use crate::SignedReleaseError;

    #[test]
    fn an_unsigned_current_installer_cannot_select_an_uninstall_target() {
        assert!(matches!(
            verify_current_uninstall_target(),
            Err(UninstallPreflightError::InstallerInvalid(
                SignedReleaseError::SignatureInvalid(SignatureError::TrustRejected)
            ))
        ));
    }
}
