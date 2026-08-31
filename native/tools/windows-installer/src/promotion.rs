//! Same-volume promotion of a fully verified private release stage.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use crate::prepared::into_promotion_parts;
use crate::staging::StagedRelease;
use crate::{PackageVersion, PreparedRelease};

mod raw;

/// A complete version directory retained before later registry publication.
pub struct PromotedRelease {
    application_id: String,
    package_root: PathBuf,
    install_record: String,
}

impl fmt::Debug for PromotedRelease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PromotedRelease")
            .field(
                "package_root_units",
                &self
                    .package_root
                    .as_os_str()
                    .to_string_lossy()
                    .encode_utf16()
                    .count(),
            )
            .field("install_record_bytes", &self.install_record.len())
            .finish_non_exhaustive()
    }
}

impl PromotedRelease {
    /// Returns the installer-validated identity for fixed policy publication.
    #[must_use]
    pub(crate) fn application_id(&self) -> &str {
        &self.application_id
    }

    /// Returns the already validated record for fixed policy publication.
    #[must_use]
    pub(crate) fn install_record(&self) -> &str {
        &self.install_record
    }
}

/// A promotion-ready release could not become a new version directory.
#[derive(Debug)]
pub enum PromotionError {
    /// The private staging directory did not have an owned application parent.
    StagingPathInvalid,
    /// The signed version directory already existed and must not be changed.
    VersionAlreadyExists,
    /// Windows could not rename the private stage to the sibling version directory.
    DirectoryMoveFailed,
}

impl fmt::Display for PromotionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::StagingPathInvalid => "the private staging path is invalid",
            Self::VersionAlreadyExists => "the release version already exists",
            Self::DirectoryMoveFailed => "Windows could not promote the release directory",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PromotionError {}

/// Promotes a fully checked stage to its new signed version directory.
///
/// This operation never accepts a destination name, replaces an existing
/// directory, copies across volumes, writes registry policy, launches a
/// process, or downloads content. It requires `PreparedRelease`, which is
/// available only after the current installer and staged executable publisher
/// gates both passed.
pub fn promote_prepared_release(
    prepared: PreparedRelease,
) -> Result<PromotedRelease, PromotionError> {
    let (staged, version, application_id) = into_promotion_parts(prepared);
    promote_staged_release(staged, version, application_id)
}

fn promote_staged_release(
    staged: StagedRelease,
    version: PackageVersion,
    application_id: String,
) -> Result<PromotedRelease, PromotionError> {
    let destination = version_destination(staged.package_root(), version)?;
    if destination
        .try_exists()
        .map_err(|_| PromotionError::StagingPathInvalid)?
    {
        return Err(PromotionError::VersionAlreadyExists);
    }
    raw::move_directory(staged.package_root(), &destination)?;
    let (package_root, install_record) = staged.into_promoted_parts(destination);
    Ok(PromotedRelease {
        application_id,
        package_root,
        install_record,
    })
}

fn version_destination(
    staging_path: &Path,
    version: PackageVersion,
) -> Result<PathBuf, PromotionError> {
    let parent = staging_path
        .parent()
        .ok_or(PromotionError::StagingPathInvalid)?;
    Ok(parent.join(format!(
        "{}.{}.{}",
        version.major(),
        version.minor(),
        version.patch()
    )))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use anodrel_application::sha256;
    use anodrel_release_bundle::{BundleEntryInput, encode};

    use crate::staging::stage_checked_release;
    use crate::{PromotionError, ReleaseManifest, verify_bundle};

    use super::promote_staged_release;

    const PUBLISHER: &str = "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585";

    #[test]
    fn a_stage_moves_to_its_new_signed_version_without_becoming_disposable() {
        let parent = TemporaryDirectory::new();
        let (manifest, staged) = staged_fixture(parent.path());
        let stage_root = staged.package_root().to_path_buf();

        let promoted = promote_staged_release(
            staged,
            manifest.package_version(),
            manifest.application_id().to_owned(),
        )
        .expect("a unique sibling version receives the checked stage");
        let destination = parent.path().join("1.2.3");
        assert!(!stage_root.exists());
        assert!(destination.is_dir());
        assert_eq!(
            std::fs::read(destination.join("content/main.txt")).expect("content is retained"),
            b"promoted release content"
        );
        drop(promoted);
        assert!(
            destination.is_dir(),
            "a promoted version remains for publication"
        );
    }

    #[test]
    fn an_existing_version_is_never_replaced() {
        let parent = TemporaryDirectory::new();
        let destination = parent.path().join("1.2.3");
        std::fs::create_dir(&destination).expect("the existing version is created");
        std::fs::write(destination.join("keep.txt"), b"existing release")
            .expect("the existing version is populated");
        let (manifest, staged) = staged_fixture(parent.path());

        assert!(matches!(
            promote_staged_release(
                staged,
                manifest.package_version(),
                manifest.application_id().to_owned(),
            ),
            Err(PromotionError::VersionAlreadyExists)
        ));
        assert_eq!(
            std::fs::read(destination.join("keep.txt")).expect("the existing version remains"),
            b"existing release"
        );
        assert_eq!(
            std::fs::read_dir(parent.path())
                .expect("the staging parent is readable")
                .count(),
            1,
            "the failed private stage was cleaned up"
        );
    }

    fn staged_fixture(parent: &Path) -> (ReleaseManifest, crate::staging::StagedRelease) {
        let executable = b"promotion executable";
        let content = b"promoted release content";
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
        let staged = stage_checked_release(parent, &manifest, &bundle)
            .expect("the fixture becomes a private stage");
        (manifest, staged)
    }

    fn package_manifest(content: &[u8]) -> Vec<u8> {
        let digest = sha256::to_lower_hex(&sha256::digest(content));
        format!(
            r#"{{
  "manifestVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.promotion-test",
  "displayName": "Promotion Test",
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
  "applicationId": "org.anodrel.promotion-test",
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

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "anodrel-promotion-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("the system time is after the epoch")
                    .as_nanos()
            ));
            std::fs::create_dir(&path).expect("the staging parent is created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
