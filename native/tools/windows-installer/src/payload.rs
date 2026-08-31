//! Joined release-manifest and owned-bundle validation.

use std::fmt;

use anodrel_release_bundle::{ReleaseBundle, ReleaseBundleError};

use crate::ReleaseManifest;

/// A signed manifest and its embedded bundle did not agree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleasePayloadError {
    /// The embedded payload length differed from the signed descriptor.
    LengthMismatch,
    /// The embedded payload digest differed from the signed descriptor.
    DigestMismatch,
    /// The owned payload bytes did not meet the bundle contract.
    BundleInvalid(ReleaseBundleError),
}

impl fmt::Display for ReleasePayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::LengthMismatch => "release payload length does not match its manifest",
            Self::DigestMismatch => "release payload digest does not match its manifest",
            Self::BundleInvalid(_) => "release payload bundle is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ReleasePayloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BundleInvalid(error) => Some(error),
            Self::LengthMismatch | Self::DigestMismatch => None,
        }
    }
}

/// Verifies one complete signed payload, then parses its owned file bundle.
///
/// The returned file slices borrow `payload`; neither this operation nor the
/// bundle parser copies file contents or performs filesystem I/O.
pub fn verify_bundle<'payload>(
    manifest: &ReleaseManifest,
    payload: &'payload [u8],
) -> Result<ReleaseBundle<'payload>, ReleasePayloadError> {
    if payload.len() as u64 != manifest.payload().byte_length() {
        return Err(ReleasePayloadError::LengthMismatch);
    }
    let digest = anodrel_application::sha256::digest(payload);
    if !manifest.payload().matches_digest(digest) {
        return Err(ReleasePayloadError::DigestMismatch);
    }
    ReleaseBundle::parse(payload).map_err(ReleasePayloadError::BundleInvalid)
}
