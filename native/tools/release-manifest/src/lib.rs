#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! First-party strict release-manifest authoring from checked bundle bytes.
//!
//! This crate derives application identity and all digests from a checked owned
//! release bundle. It writes only one fresh manifest output and never signs,
//! embeds, installs, launches, downloads, or changes trust.

mod create;
mod error;
mod output;
mod plan;

pub use create::create_release_manifest;
pub use error::ReleaseManifestAuthorError;

#[cfg(test)]
mod tests;
