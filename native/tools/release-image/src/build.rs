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

/// Checks fixed release resources in one existing release-image file.
///
/// This is a read-only authoring check. It loads only PE resources as data,
/// parses the manifest, and verifies the bundle against that manifest. It does
/// not verify Authenticode, modify the image, create an output, or access a
/// certificate store.
pub fn verify_release_image(path: &Path) -> Result<(), ReleaseImageError> {
    read_release_manifest(path).map(|_| ())
}

/// Checks that one release image names exactly one expected opaque publisher.
///
/// This is a read-only authoring check that additionally compares the supplied
/// leaf certificate fingerprint to the publisher value in the checked embedded
/// release manifest. It does not expose that manifest value, verify an image
/// signature, create an output, or access a certificate store.
pub fn verify_release_image_for_publisher(
    path: &Path,
    expected_publisher: [u8; 32],
) -> Result<(), ReleaseImageError> {
    let manifest = read_release_manifest(path)?;
    manifest
        .matches_publisher_fingerprint(expected_publisher)
        .then_some(())
        .ok_or(ReleaseImageError::PublisherMismatch)
}

fn read_release_manifest(path: &Path) -> Result<ReleaseManifest, ReleaseImageError> {
    validate_image(path)?;
    let (manifest_bytes, payload) = raw::read_resources(path)?;
    let manifest_text = std::str::from_utf8(&manifest_bytes)
        .map_err(|_| ReleaseImageError::ManifestInvalid(ReleaseManifestError::Invalid))?;
    let manifest =
        ReleaseManifest::parse(manifest_text).map_err(ReleaseImageError::ManifestInvalid)?;
    verify_bundle(&manifest, &payload).map_err(ReleaseImageError::PayloadInvalid)?;
    Ok(manifest)
}

fn validate_template(path: &Path) -> Result<(), ReleaseImageError> {
    if !path.is_absolute() || !fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return Err(ReleaseImageError::TemplateInvalid);
    }
    Ok(())
}

fn validate_image(path: &Path) -> Result<(), ReleaseImageError> {
    if !path.is_absolute() || !fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return Err(ReleaseImageError::ImageInvalid);
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
