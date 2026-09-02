#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Direct Windows CMS signing and exact-publisher verification for catalogues.
//!
//! This adapter composes strict `anodrel.update-catalogue.v1` parsing with one
//! attached CMS signature from the exact current-user certificate and one
//! exact-publisher verification. It performs no file, network, registry,
//! process, elevation, installation, or update operation.

mod error;
mod signature;

pub use error::UpdateCatalogueSignatureError;
pub use signature::{VerifiedUpdateCatalogue, sign_update_catalogue, verify_update_catalogue};

/// Maximum attached CMS envelope size for one version-1 update catalogue.
pub const MAX_SIGNED_UPDATE_CATALOGUE_BYTES: usize = 128 * 1024;

#[cfg(test)]
mod tests;
