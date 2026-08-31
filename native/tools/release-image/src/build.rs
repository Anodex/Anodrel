//! Validated release-image assembly flow.

use std::{fs, path::Path};

use anodrel_windows_installer::{ReleaseManifest, ReleaseManifestError, verify_bundle};

use crate::{ReleaseImageError, raw};

/// Embeds one checked release into a new copy of an installer template.
///
/// `template` and `output` must be absolute paths. The output must not exist;
/// this operation never overwrites a file or alters its source template. The
/// result is deliberately unsigned because resource modification must happen
/// before the separate production signing step.
pub fn embed_release_image(
    template: &Path,
    output: &Path,
    manifest_bytes: &[u8],
    payload: &[u8],
) -> Result<(), ReleaseImageError> {
    validate_template(template)?;
    validate_output(output)?;
    let manifest_text = std::str::from_utf8(manifest_bytes)
        .map_err(|_| ReleaseImageError::ManifestInvalid(ReleaseManifestError::Invalid))?;
    let manifest =
        ReleaseManifest::parse(manifest_text).map_err(ReleaseImageError::ManifestInvalid)?;
    verify_bundle(&manifest, payload).map_err(ReleaseImageError::PayloadInvalid)?;

    fs::copy(template, output).map_err(|_| ReleaseImageError::CopyFailed)?;
    raw::write_resources(output, manifest_bytes, payload)?;
    raw::resources_match(output, manifest_bytes, payload)?;
    Ok(())
}

fn validate_template(path: &Path) -> Result<(), ReleaseImageError> {
    if !path.is_absolute() || !fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return Err(ReleaseImageError::TemplateInvalid);
    }
    Ok(())
}

fn validate_output(path: &Path) -> Result<(), ReleaseImageError> {
    if !path.is_absolute() {
        return Err(ReleaseImageError::OutputInvalid);
    }
    if path
        .try_exists()
        .map_err(|_| ReleaseImageError::OutputInvalid)?
    {
        return Err(ReleaseImageError::OutputAlreadyExists);
    }
    let parent = path.parent().ok_or(ReleaseImageError::OutputInvalid)?;
    if !fs::metadata(parent).is_ok_and(|metadata| metadata.is_dir()) {
        return Err(ReleaseImageError::OutputInvalid);
    }
    Ok(())
}
