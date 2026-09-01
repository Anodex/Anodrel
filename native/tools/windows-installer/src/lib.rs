#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! First-party Windows installer foundations.
//!
//! This crate validates the private release carried by a signed Windows
//! installer and can prepare it in a private staging directory. It does not
//! promote a version directory, change the registry, install a certificate, or
//! launch an application.

mod error;
#[cfg(windows)]
mod installation;
#[cfg(windows)]
mod machine_root;
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
#[cfg(windows)]
mod uninstall;
#[cfg(windows)]
mod update;
#[cfg(windows)]
mod update_installation;

pub use error::ReleaseManifestError;
#[cfg(windows)]
pub use installation::{InstallCurrentError, InstalledRelease, install_current_signed_release};
#[cfg(windows)]
pub use machine_root::MachineRootError;
pub use manifest::{PackageVersion, PayloadDescriptor, ReleaseManifest};
pub use payload::{ReleasePayloadError, verify_bundle};
#[cfg(windows)]
pub use prepared::{PreparedRelease, PreparedReleaseError, prepare_current_signed_release};
#[cfg(windows)]
pub use promotion::{PromotedRelease, PromotionError, promote_prepared_release};
#[cfg(windows)]
pub use publication::{
    PublicationError, PublishedRelease, PublishedUpdate, UpdatePublicationError,
    publish_promoted_release, publish_promoted_update,
};
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
#[cfg(windows)]
pub use uninstall::{
    PolicyRemovedUninstallTarget, UninstallPackageRemovalError, UninstallPolicyRemovalError,
    UninstallPreflightError, VerifiedUninstallTarget, remove_policy_removed_package,
    remove_verified_uninstall_policy, verify_current_uninstall_target,
};
#[cfg(windows)]
pub use update::{UpdatePreflightError, VerifiedUpdateCandidate, verify_current_update_candidate};
#[cfg(windows)]
pub use update_installation::{UpdateCurrentError, UpdatedRelease, update_current_signed_release};

/// Maximum UTF-8 release-manifest size before JSON parsing.
pub const MAX_RELEASE_MANIFEST_BYTES: usize = 16 * 1024;

/// Maximum uncompressed installer payload declared by version 1.0.
pub const MAX_PAYLOAD_BYTES: u64 = anodrel_release_bundle::MAX_BUNDLE_BYTES as u64;

#[cfg(test)]
mod tests;
