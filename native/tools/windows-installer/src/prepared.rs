//! Staged executable publisher verification before promotion.

use std::{fmt, path::Path};

use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::installed_uninstaller::{InstalledUninstallerError, stage_current_installer_image};
use crate::staging::{StagedRelease, stage_checked_release};
use crate::{
    PackageVersion, ReleaseManifest, SignedReleaseError, StagedReleaseError,
    VerifiedEmbeddedRelease, verify_current_signed_release,
};

/// A private release stage that passed installer and executable publisher checks.
pub struct PreparedRelease {
    staged: StagedRelease,
    version: PackageVersion,
    application_id: String,
}

impl fmt::Debug for PreparedRelease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PreparedRelease")
            .field(&self.staged)
            .field(&self.version)
            .finish()
    }
}

/// A signed release could not become a promotion-ready private package.
#[derive(Debug)]
pub enum PreparedReleaseError {
    /// The current installer did not pass its signed embedded-release gate.
    InstallerInvalid(SignedReleaseError),
    /// Checked files could not become a private valid package stage.
    StagingInvalid(StagedReleaseError),
    /// The current signed installer could not become the fixed staged uninstaller.
    UninstallerStagingFailed(InstalledUninstallerError),
    /// Windows did not accept the staged executable's signature.
    ExecutableSignatureInvalid(SignatureError),
    /// The staged executable signer differed from the embedded release publisher.
    ExecutablePublisherMismatch,
    /// Windows did not accept the staged product launcher's signature.
    LauncherSignatureInvalid(SignatureError),
    /// The staged product launcher signer differed from the release publisher.
    LauncherPublisherMismatch,
}

impl fmt::Display for PreparedReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InstallerInvalid(_) => "the signed installer release is invalid",
            Self::StagingInvalid(_) => "the release could not be staged safely",
            Self::UninstallerStagingFailed(_) => {
                "the installed uninstaller image could not be prepared"
            }
            Self::ExecutableSignatureInvalid(_) => {
                "Windows did not accept the staged executable signature"
            }
            Self::ExecutablePublisherMismatch => {
                "the staged executable publisher does not match the release"
            }
            Self::LauncherSignatureInvalid(_) => {
                "Windows did not accept the staged product launcher signature"
            }
            Self::LauncherPublisherMismatch => {
                "the staged product launcher publisher does not match the release"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PreparedReleaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstallerInvalid(error) => Some(error),
            Self::StagingInvalid(error) => Some(error),
            Self::UninstallerStagingFailed(error) => Some(error),
            Self::ExecutableSignatureInvalid(error) => Some(error),
            Self::LauncherSignatureInvalid(error) => Some(error),
            Self::ExecutablePublisherMismatch | Self::LauncherPublisherMismatch => None,
        }
    }
}

/// Creates a promotion-ready private stage from the current signed installer.
///
/// The caller supplies only an installer-owned staging parent. This operation
/// verifies the current installer, extracts its checked release into a new
/// private directory, validates its package record, and verifies the contained
/// executable with Windows Authenticode. It does not promote, publish, launch,
/// download, or retain a stage after the returned value is dropped.
pub fn prepare_current_signed_release(
    staging_parent: &Path,
) -> Result<PreparedRelease, PreparedReleaseError> {
    let release =
        verify_current_signed_release().map_err(PreparedReleaseError::InstallerInvalid)?;
    prepare_verified_signed_release(staging_parent, release)
}

/// Prepares one release that has just passed the current-installer signature gate.
///
/// This internal composition accepts the verified release token rather than a
/// manifest, bundle, executable, or publisher supplied by a caller. It still
/// privately stages the release, validates its installed record, and verifies
/// the staged executable before returning a promotion-ready result.
pub(crate) fn prepare_verified_signed_release(
    staging_parent: &Path,
    release: VerifiedEmbeddedRelease<'_>,
) -> Result<PreparedRelease, PreparedReleaseError> {
    let manifest = release.release().manifest();
    let version = manifest.package_version();
    let application_id = manifest.application_id().to_owned();
    let staged = stage_checked_release(staging_parent, manifest, release.release().bundle())
        .map_err(PreparedReleaseError::StagingInvalid)?;
    stage_current_installer_image(&staged, manifest)
        .map_err(PreparedReleaseError::UninstallerStagingFailed)?;
    verify_staged_images(&staged, manifest)?;
    Ok(PreparedRelease {
        staged,
        version,
        application_id,
    })
}

/// Transfers a fully checked stage to the owned promotion boundary.
pub(crate) fn into_promotion_parts(
    prepared: PreparedRelease,
) -> (StagedRelease, PackageVersion, String) {
    (prepared.staged, prepared.version, prepared.application_id)
}

fn verify_staged_images(
    staged: &StagedRelease,
    manifest: &ReleaseManifest,
) -> Result<(), PreparedReleaseError> {
    let signer = verify_embedded_signature(staged.executable_path())
        .map_err(PreparedReleaseError::ExecutableSignatureInvalid)?;
    manifest
        .matches_publisher_fingerprint(signer.as_bytes())
        .then_some(())
        .ok_or(PreparedReleaseError::ExecutablePublisherMismatch)?;
    if let Some(launcher_path) = staged.product_launcher_path() {
        let signer = verify_embedded_signature(launcher_path)
            .map_err(PreparedReleaseError::LauncherSignatureInvalid)?;
        manifest
            .matches_publisher_fingerprint(signer.as_bytes())
            .then_some(())
            .ok_or(PreparedReleaseError::LauncherPublisherMismatch)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use anodrel_application::sha256;
    use anodrel_release_bundle::{BundleEntryInput, encode};
    use anodrel_windows_signature::SignatureError;

    use crate::staging::stage_checked_release;
    use crate::test_support::TestDirectory;
    use crate::{ReleaseManifest, verify_bundle};

    use super::{PreparedReleaseError, verify_staged_images};

    const PUBLISHER: &str = "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585";

    #[test]
    fn an_unsigned_staged_executable_cannot_be_prepared_for_promotion() {
        let parent = TestDirectory::new("prepared-release");
        let executable = b"not a signed executable";
        let content = b"prepared release content";
        let package = package_manifest(content);
        let payload = encode(&[
            BundleEntryInput {
                path: "anodrel.application.json",
                contents: &package,
            },
            BundleEntryInput {
                path: "bin/Product.exe",
                contents: executable,
            },
            BundleEntryInput {
                path: "content/main.txt",
                contents: content,
            },
        ])
        .expect("the fixture bundle encodes");
        let manifest = ReleaseManifest::parse(&release_manifest(&payload, executable))
            .expect("the fixture manifest is valid");
        let bundle = verify_bundle(&manifest, &payload).expect("the fixture bundle is valid");
        let staged = stage_checked_release(parent.path(), &manifest, &bundle)
            .expect("the checked fixture stages before signer validation");

        assert!(matches!(
            verify_staged_images(&staged, &manifest),
            Err(PreparedReleaseError::ExecutableSignatureInvalid(
                SignatureError::TrustRejected
            ))
        ));
    }

    fn package_manifest(content: &[u8]) -> Vec<u8> {
        let digest = sha256::to_lower_hex(&sha256::digest(content));
        format!(
            r#"{{
  "manifestVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.prepared-test",
  "displayName": "Prepared Test",
  "content": {{
    "format": "anodrel.text.v1",
    "path": "content/main.txt",
    "sha256": "{digest}"
  }}
}}"#
        )
        .into_bytes()
    }

    fn release_manifest(payload: &[u8], executable: &[u8]) -> String {
        let payload_digest = sha256::to_lower_hex(&sha256::digest(payload));
        let executable_digest = sha256::to_lower_hex(&sha256::digest(executable));
        format!(
            r#"{{
  "formatVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.prepared-test",
  "packageVersion": {{ "major": 1, "minor": 2, "patch": 3 }},
  "executable": {{ "path": "bin/Product.exe", "sha256": "{executable_digest}" }},
  "publisher": {{ "leafCertificateSha256": "{PUBLISHER}" }},
  "capabilities": [],
  "networkOrigins": [],
  "payload": {{ "byteLength": {}, "sha256": "{payload_digest}" }}
}}"#,
            payload.len()
        )
    }
}
