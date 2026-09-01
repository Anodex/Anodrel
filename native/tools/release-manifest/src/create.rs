//! Checked bundle-to-manifest authoring composition.

use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

use anodrel_application::{ApplicationManifest, sha256, validate_text_content};
use anodrel_release_bundle::{MAX_BUNDLE_BYTES, ReleaseBundle};
use anodrel_windows_installer::{MAX_RELEASE_MANIFEST_BYTES, ReleaseManifest, verify_bundle};

use crate::{ReleaseManifestAuthorError, output, plan::ReleasePlan};

const APPLICATION_MANIFEST_PATH: &str = "anodrel.application.json";

/// Creates one strict final release manifest from an explicit plan and bundle.
///
/// Every path must be absolute. The plan and bundle must be normal regular
/// files. The final output must be a previously absent file with a normal parent
/// directory. Identity, executable digest, application-content check, and
/// payload facts all come from the checked bundle; the output is written only
/// after the derived manifest re-parses and verifies against those same bytes.
pub fn create_release_manifest(
    plan_path: &Path,
    bundle_path: &Path,
    output_path: &Path,
) -> Result<(), ReleaseManifestAuthorError> {
    let plan_text = read_input(plan_path, MAX_RELEASE_MANIFEST_BYTES)?;
    let plan = ReleasePlan::parse(
        std::str::from_utf8(&plan_text).map_err(|_| ReleaseManifestAuthorError::PlanInvalid)?,
    )?;
    let bundle_bytes = read_input(bundle_path, MAX_BUNDLE_BYTES)?;
    let bundle =
        ReleaseBundle::parse(&bundle_bytes).map_err(ReleaseManifestAuthorError::BundleInvalid)?;
    let application = checked_application(&bundle)?;
    let executable = bundle
        .file(plan.executable_path())
        .ok_or(ReleaseManifestAuthorError::ExecutableUnavailable)?;
    let manifest_text = plan.render(
        application.identity().application_id(),
        sha256::digest(executable),
        &bundle_bytes,
    );
    let manifest = ReleaseManifest::parse(&manifest_text)
        .map_err(ReleaseManifestAuthorError::ManifestInvalid)?;
    verify_bundle(&manifest, &bundle_bytes).map_err(ReleaseManifestAuthorError::PayloadInvalid)?;
    output::write_new_manifest(output_path, manifest_text.as_bytes())
}

fn checked_application(
    bundle: &ReleaseBundle<'_>,
) -> Result<ApplicationManifest, ReleaseManifestAuthorError> {
    let manifest_bytes = bundle
        .file(APPLICATION_MANIFEST_PATH)
        .ok_or(ReleaseManifestAuthorError::ApplicationManifestUnavailable)?;
    let manifest_text = std::str::from_utf8(manifest_bytes)
        .map_err(|_| ReleaseManifestAuthorError::ApplicationContentInvalid)?;
    let manifest = ApplicationManifest::parse(manifest_text)
        .map_err(ReleaseManifestAuthorError::ApplicationManifestInvalid)?;
    let content = bundle
        .file(manifest.content_path())
        .ok_or(ReleaseManifestAuthorError::ApplicationContentInvalid)?;
    if !manifest.matches_content_digest(sha256::digest(content)) {
        return Err(ReleaseManifestAuthorError::ApplicationContentInvalid);
    }
    let text = std::str::from_utf8(content)
        .map_err(|_| ReleaseManifestAuthorError::ApplicationContentInvalid)?;
    validate_text_content(text)
        .map_err(|_| ReleaseManifestAuthorError::ApplicationContentInvalid)?;
    Ok(manifest)
}

fn read_input(path: &Path, maximum: usize) -> Result<Vec<u8>, ReleaseManifestAuthorError> {
    if !path.is_absolute() {
        return Err(ReleaseManifestAuthorError::InputInvalid);
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| ReleaseManifestAuthorError::InputReadFailed)?;
    if !metadata.is_file() || is_link_like(&metadata) {
        return Err(ReleaseManifestAuthorError::InputInvalid);
    }
    if metadata.len() > maximum as u64 {
        return Err(ReleaseManifestAuthorError::InputReadFailed);
    }
    let file = File::open(path).map_err(|_| ReleaseManifestAuthorError::InputReadFailed)?;
    let capacity =
        usize::try_from(metadata.len()).map_err(|_| ReleaseManifestAuthorError::InputReadFailed)?;
    let mut reader = file.take(maximum as u64 + 1);
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| ReleaseManifestAuthorError::InputReadFailed)?;
    if bytes.len() > maximum {
        return Err(ReleaseManifestAuthorError::InputReadFailed);
    }
    Ok(bytes)
}

fn is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        metadata.file_attributes() & 0x0400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
