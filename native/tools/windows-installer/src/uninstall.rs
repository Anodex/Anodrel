//! Signed identity preflight for a later owned uninstall transaction.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{
    PackageVersion, SignedReleaseError, installed_uninstaller::installed_uninstaller_path,
    verify_current_signed_release,
};

/// A signed, policy-selected application eligible for later uninstall work.
pub struct VerifiedUninstallTarget {
    application_id: String,
    package_root: PathBuf,
}

impl VerifiedUninstallTarget {
    #[must_use]
    pub(crate) fn application_id(&self) -> &str {
        &self.application_id
    }
}

/// A verified uninstall target whose fixed machine record was removed.
pub struct PolicyRemovedUninstallTarget {
    target: VerifiedUninstallTarget,
}

impl fmt::Debug for PolicyRemovedUninstallTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PolicyRemovedUninstallTarget")
            .field(&self.target)
            .finish()
    }
}

impl PolicyRemovedUninstallTarget {
    #[must_use]
    pub(crate) fn package_root(&self) -> &std::path::Path {
        &self.target.package_root
    }
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
    /// The selected package root did not use the signed release version name.
    SelectedVersionInvalid,
    /// The current signed image was not the selected fixed installed uninstaller.
    CurrentImageNotSelected,
}

/// A fixed policy record could not be removed after verified preflight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UninstallPolicyRemovalError {
    /// Windows denied the elevated fixed policy change.
    AccessDenied,
    /// The fixed policy record was unavailable or could not be removed safely.
    PolicyUnavailable,
}

impl fmt::Display for UninstallPolicyRemovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AccessDenied => "machine policy cannot be changed; run from an elevated shell",
            Self::PolicyUnavailable => "the installed application policy could not be removed",
        })
    }
}
impl std::error::Error for UninstallPolicyRemovalError {}

/// A policy-removed package tree could not be deleted safely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UninstallPackageRemovalError {
    /// A link or junction was encountered and cleanup refused to traverse it.
    ReparsePointRefused,
    /// Windows could not remove the verified package tree.
    PackageRemovalFailed,
}

impl fmt::Display for UninstallPackageRemovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ReparsePointRefused => "package cleanup refused a reparse point",
            Self::PackageRemovalFailed => "the installed package could not be removed",
        })
    }
}
impl std::error::Error for UninstallPackageRemovalError {}

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
            Self::SelectedVersionInvalid => {
                "the selected package version does not match the installer release"
            }
            Self::CurrentImageNotSelected => {
                "the current installer is not the selected installed uninstaller"
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
            Self::InstalledPublisherMismatch
            | Self::InstallerPublisherMismatch
            | Self::SelectedVersionInvalid
            | Self::CurrentImageNotSelected => None,
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
    selected_version(installed.package_root())
        .is_some_and(|version| version == manifest.package_version())
        .then_some(())
        .ok_or(UninstallPreflightError::SelectedVersionInvalid)?;
    current_image_is_selected_uninstaller(installed.package_root())?;
    Ok(VerifiedUninstallTarget {
        application_id: manifest.application_id().to_owned(),
        package_root: installed.package_root().to_path_buf(),
    })
}

fn selected_version(package_root: &Path) -> Option<PackageVersion> {
    package_root
        .file_name()?
        .to_str()
        .and_then(PackageVersion::from_canonical_directory_name)
}

fn current_image_is_selected_uninstaller(
    package_root: &Path,
) -> Result<(), UninstallPreflightError> {
    let expected = installed_uninstaller_path(package_root);
    let expected_metadata = fs::symlink_metadata(&expected)
        .map_err(|_| UninstallPreflightError::CurrentImageNotSelected)?;
    if expected_metadata.file_type().is_symlink() || !expected_metadata.is_file() {
        return Err(UninstallPreflightError::CurrentImageNotSelected);
    }
    let expected =
        fs::canonicalize(expected).map_err(|_| UninstallPreflightError::CurrentImageNotSelected)?;
    if !expected.starts_with(package_root) {
        return Err(UninstallPreflightError::CurrentImageNotSelected);
    }
    let current = std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|_| UninstallPreflightError::CurrentImageNotSelected)?;
    (current == expected)
        .then_some(())
        .ok_or(UninstallPreflightError::CurrentImageNotSelected)
}

/// Removes only the fixed machine `record` value for a verified target.
pub fn remove_verified_uninstall_policy(
    target: VerifiedUninstallTarget,
) -> Result<PolicyRemovedUninstallTarget, UninstallPolicyRemovalError> {
    raw::remove_record(target.application_id())?;
    Ok(PolicyRemovedUninstallTarget { target })
}

/// Removes only the verified package tree after fixed policy removal.
pub fn remove_policy_removed_package(
    target: PolicyRemovedUninstallTarget,
) -> Result<(), UninstallPackageRemovalError> {
    crate::recovery::raw::remove_normal_tree_except_installer(target.package_root()).map_err(
        |error| match error {
            crate::RecoveryCleanupError::ReparsePointRefused => {
                UninstallPackageRemovalError::ReparsePointRefused
            }
            crate::RecoveryCleanupError::DiscoveryFailed(_)
            | crate::RecoveryCleanupError::RemovalFailed => {
                UninstallPackageRemovalError::PackageRemovalFailed
            }
        },
    )?;
    crate::recovery::raw::schedule_installer_tree_removal(target.package_root()).map_err(|error| {
        match error {
            crate::RecoveryCleanupError::ReparsePointRefused => {
                UninstallPackageRemovalError::ReparsePointRefused
            }
            crate::RecoveryCleanupError::DiscoveryFailed(_)
            | crate::RecoveryCleanupError::RemovalFailed => {
                UninstallPackageRemovalError::PackageRemovalFailed
            }
        }
    })
}

mod raw {
    use super::UninstallPolicyRemovalError;
    type HKey = isize;
    const HKEY_LOCAL_MACHINE: HKey = 0x8000_0002_usize as HKey;
    const KEY_SET_VALUE: u32 = 0x0002;
    const KEY_WOW64_64KEY: u32 = 0x0100;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_ACCESS_DENIED: i32 = 5;
    const POLICY_PREFIX: &str = "Software\\Anodrel\\Applications\\";
    #[link(name = "Advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            key: HKey,
            sub_key: *const u16,
            options: u32,
            access: u32,
            result: *mut HKey,
        ) -> i32;
        fn RegDeleteValueW(key: HKey, value_name: *const u16) -> i32;
        fn RegCloseKey(key: HKey) -> i32;
    }
    pub(super) fn remove_record(application_id: &str) -> Result<(), UninstallPolicyRemovalError> {
        let path = wide(&format!("{POLICY_PREFIX}{application_id}"));
        let mut key = 0_isize;
        // SAFETY: The fixed path is NUL terminated and `key` is one HKEY output slot.
        let opened = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                path.as_ptr(),
                0,
                KEY_SET_VALUE | KEY_WOW64_64KEY,
                &mut key,
            )
        };
        if opened != ERROR_SUCCESS {
            return Err(error(opened));
        }
        let guard = Key(key);
        let value = wide("record");
        // SAFETY: The guard owns the fixed machine key and value is NUL terminated.
        let status = unsafe { RegDeleteValueW(guard.0, value.as_ptr()) };
        (status == ERROR_SUCCESS).then_some(()).ok_or(error(status))
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn error(status: i32) -> UninstallPolicyRemovalError {
        if status == ERROR_ACCESS_DENIED {
            UninstallPolicyRemovalError::AccessDenied
        } else {
            UninstallPolicyRemovalError::PolicyUnavailable
        }
    }
    struct Key(HKey);
    impl Drop for Key {
        fn drop(&mut self) {
            /* SAFETY: owns a successful registry handle. */
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{UninstallPreflightError, selected_version, verify_current_uninstall_target};
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

    #[test]
    fn selected_uninstaller_version_uses_only_a_canonical_package_directory() {
        assert_eq!(
            selected_version(std::path::Path::new("C:\\Program Files\\Anodrel\\1.2.3")),
            Some(crate::PackageVersion::new(1, 2, 3))
        );
        assert!(selected_version(std::path::Path::new("C:\\Anodrel\\1.02.3")).is_none());
    }
}
