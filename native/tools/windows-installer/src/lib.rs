#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! First-party Windows installer foundations.
//!
//! This crate parses the private release manifest that a later signed Windows
//! installer will embed. It does not extract a payload, change the registry,
//! create a directory, install a certificate, or launch an application.

mod error;
mod manifest;
mod record;

pub use error::ReleaseManifestError;
pub use manifest::{PackageVersion, PayloadDescriptor, ReleaseManifest};

/// Maximum UTF-8 release-manifest size before JSON parsing.
pub const MAX_RELEASE_MANIFEST_BYTES: usize = 16 * 1024;

/// Maximum uncompressed installer payload declared by version 1.0.
pub const MAX_PAYLOAD_BYTES: u64 = 512 * 1024 * 1024;

#[cfg(test)]
mod tests;
