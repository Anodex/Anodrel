#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Bounded owned release-bundle encoding and parsing.
//!
//! This crate operates only on bytes supplied by a signed installer resource.
//! It does not read or write files, follow paths, unpack a directory, verify an
//! executable signature, or mutate machine policy.

mod error;
mod format;

pub use error::ReleaseBundleError;
pub use format::{BundleEntry, BundleEntryInput, ReleaseBundle, encode};

/// Largest accepted complete version-1 bundle payload.
pub const MAX_BUNDLE_BYTES: usize = 512 * 1024 * 1024;

/// Largest number of regular files in one version-1 bundle.
pub const MAX_BUNDLE_ENTRIES: usize = 128;

/// Largest UTF-8 path byte length in one version-1 bundle entry.
pub const MAX_BUNDLE_PATH_BYTES: usize = 240;

#[cfg(test)]
mod tests;
