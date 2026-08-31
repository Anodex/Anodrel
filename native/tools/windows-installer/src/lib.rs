#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! First-party Windows installer foundations.
//!
//! This crate validates the private release carried by a signed Windows
//! installer and can prepare it in a private staging directory. It does not
//! promote a version directory, change the registry, install a certificate, or
//! launch an application.

mod error;
mod manifest;
mod payload;
#[cfg(windows)]
mod prepared;
#[cfg(windows)]
mod promotion;
#[cfg(windows)]
mod publication;
mod record;
mod recovery;
#[cfg(windows)]
mod resources;
#[cfg(windows)]
mod signing;
mod staging;

pub use error::ReleaseManifestError;
pub use manifest::{PackageVersion, PayloadDescriptor, ReleaseManifest};
pub use payload::{ReleasePayloadError, verify_bundle};
#[cfg(windows)]
pub use prepared::{PreparedRelease, PreparedReleaseError, prepare_current_signed_release};
#[cfg(windows)]
pub use promotion::{PromotedRelease, PromotionError, promote_prepared_release};
#[cfg(windows)]
pub use publication::{PublicationError, PublishedRelease, publish_promoted_release};
#[cfg(windows)]
pub use recovery::{RecoveryCleanupError, cleanup_private_stages};
pub use recovery::{RecoveryDiscoveryError, discover_private_stages};
#[cfg(windows)]
pub use resources::{
    EmbeddedRelease, EmbeddedReleaseError, RELEASE_MANIFEST_RESOURCE_ID,
    RELEASE_PAYLOAD_RESOURCE_ID, read_current_release,
};
#[cfg(windows)]
pub use signing::{SignedReleaseError, VerifiedEmbeddedRelease, verify_current_signed_release};
pub use staging::StagedReleaseError;

/// Maximum UTF-8 release-manifest size before JSON parsing.
pub const MAX_RELEASE_MANIFEST_BYTES: usize = 16 * 1024;

/// Maximum uncompressed installer payload declared by version 1.0.
pub const MAX_PAYLOAD_BYTES: u64 = anodrel_release_bundle::MAX_BUNDLE_BYTES as u64;

#[cfg(test)]
mod tests;
