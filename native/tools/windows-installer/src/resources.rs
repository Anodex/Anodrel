//! Fixed release-resource loading from the current Windows installer image.

use std::{fmt, path::Path};

use anodrel_release_bundle::ReleaseBundle;

use crate::{ReleaseManifest, ReleaseManifestError, ReleasePayloadError, verify_bundle};

mod raw;

/// The fixed `RT_RCDATA` identifier for the strict release manifest.
pub const RELEASE_MANIFEST_RESOURCE_ID: u16 = 0xA141;

/// The fixed `RT_RCDATA` identifier for the owned release-bundle payload.
pub const RELEASE_PAYLOAD_RESOURCE_ID: u16 = 0xA142;

/// A checked manifest and bundle borrowed from the current installer image.
#[derive(Debug)]
pub struct EmbeddedRelease<'image> {
    manifest: ReleaseManifest,
    bundle: ReleaseBundle<'image>,
}

/// A checked release from an external installer image held against writes.
///
/// This private resource mapping does not run the candidate. It remains alive
/// for the value's lifetime so later native handoff cannot observe a writable
/// gap between resource validation and launch.
pub(crate) struct LockedEmbeddedRelease {
    image: raw::LockedResourceImage,
    manifest: ReleaseManifest,
}

impl LockedEmbeddedRelease {
    /// Returns the signed manifest facts held by this locked image.
    pub(crate) const fn manifest(&self) -> &ReleaseManifest {
        let _ = &self.image;
        &self.manifest
    }
}

impl<'image> EmbeddedRelease<'image> {
    /// Returns the checked release manifest selected from the current image.
    #[must_use]
    pub const fn manifest(&self) -> &ReleaseManifest {
        &self.manifest
    }

    /// Returns the checked owned bundle selected from the current image.
    #[must_use]
    pub const fn bundle(&self) -> &ReleaseBundle<'image> {
        &self.bundle
    }
}

/// A safe failure category while reading the current installer release bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddedReleaseError {
    /// Windows did not provide a handle for the current executable image.
    CurrentImageUnavailable,
    /// Windows could not map an external installer image as a locked resource.
    ImageUnavailable,
    /// A fixed required resource was absent or empty.
    ResourceUnavailable,
    /// The manifest bytes were not valid strict UTF-8.
    ManifestTextInvalid,
    /// The manifest did not meet the owned release contract.
    ManifestInvalid(ReleaseManifestError),
    /// The payload did not meet the signed release contract.
    PayloadInvalid(ReleasePayloadError),
}

impl fmt::Display for EmbeddedReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CurrentImageUnavailable => "the current installer image is unavailable",
            Self::ImageUnavailable => "the installer image is unavailable",
            Self::ResourceUnavailable => "the installer release resource is unavailable",
            Self::ManifestTextInvalid => "the installer release manifest is not UTF-8",
            Self::ManifestInvalid(_) => "the installer release manifest is invalid",
            Self::PayloadInvalid(_) => "the installer release payload is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for EmbeddedReleaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ManifestInvalid(error) => Some(error),
            Self::PayloadInvalid(error) => Some(error),
            Self::CurrentImageUnavailable
            | Self::ImageUnavailable
            | Self::ResourceUnavailable
            | Self::ManifestTextInvalid => None,
        }
    }
}

/// Loads and verifies fixed release resources from the current executable image.
///
/// Resource bytes remain valid for the executable image lifetime, so the bundle
/// borrows them without a second payload allocation. This does not verify the
/// executable signature; the signed-installer activation gate must do that
/// separately before it lets this release change machine state.
pub fn read_current_release() -> Result<EmbeddedRelease<'static>, EmbeddedReleaseError> {
    let manifest_bytes = raw::current_resource(RELEASE_MANIFEST_RESOURCE_ID)?;
    let manifest = parse_manifest(manifest_bytes)?;
    let payload = raw::current_resource(RELEASE_PAYLOAD_RESOURCE_ID)?;
    let bundle = verify_bundle(&manifest, payload).map_err(EmbeddedReleaseError::PayloadInvalid)?;
    Ok(EmbeddedRelease { manifest, bundle })
}

/// Loads and validates an external image without executing it.
///
/// The returned value owns Windows' exclusive-write resource mapping. It does
/// not verify Authenticode; the signed-image activation gate performs that
/// second check while this mapping remains alive.
pub(crate) fn lock_and_read_release(
    path: &Path,
) -> Result<LockedEmbeddedRelease, EmbeddedReleaseError> {
    let image = raw::LockedResourceImage::open(path)?;
    let manifest = parse_manifest(image.resource(RELEASE_MANIFEST_RESOURCE_ID)?)?;
    verify_payload(&manifest, image.resource(RELEASE_PAYLOAD_RESOURCE_ID)?)?;
    Ok(LockedEmbeddedRelease { image, manifest })
}

fn parse_manifest(bytes: &[u8]) -> Result<ReleaseManifest, EmbeddedReleaseError> {
    let text = std::str::from_utf8(bytes).map_err(|_| EmbeddedReleaseError::ManifestTextInvalid)?;
    ReleaseManifest::parse(text).map_err(EmbeddedReleaseError::ManifestInvalid)
}

fn verify_payload(manifest: &ReleaseManifest, payload: &[u8]) -> Result<(), EmbeddedReleaseError> {
    verify_bundle(manifest, payload)
        .map(|_| ())
        .map_err(EmbeddedReleaseError::PayloadInvalid)
}

#[cfg(test)]
mod tests {
    use super::{
        EmbeddedReleaseError, RELEASE_MANIFEST_RESOURCE_ID, RELEASE_PAYLOAD_RESOURCE_ID,
        read_current_release,
    };

    #[test]
    fn release_resource_identifiers_are_fixed_and_distinct() {
        assert_eq!(RELEASE_MANIFEST_RESOURCE_ID, 0xA141);
        assert_eq!(RELEASE_PAYLOAD_RESOURCE_ID, 0xA142);
        assert_ne!(RELEASE_MANIFEST_RESOURCE_ID, RELEASE_PAYLOAD_RESOURCE_ID);
    }

    #[test]
    fn a_normal_test_image_without_release_resources_fails_closed() {
        assert!(matches!(
            read_current_release(),
            Err(EmbeddedReleaseError::ResourceUnavailable)
        ));
    }
}
