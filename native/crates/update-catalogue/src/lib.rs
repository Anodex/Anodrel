#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Strict, bounded facts for one future owned Windows update candidate.
//!
//! This crate parses a single release catalogue and compares it to known
//! installed-release facts. It performs no signature check, network I/O, file
//! I/O, machine mutation, process launch, or update installation.

mod catalogue;
mod error;

pub use catalogue::{UpdateCatalogue, UpdateInstaller};
pub use error::UpdateCatalogueError;

/// Maximum accepted UTF-8 `anodrel.update-catalogue.v1` byte length.
pub const MAX_UPDATE_CATALOGUE_BYTES: usize = 16 * 1024;

/// Maximum accepted signed installer-image byte length.
pub const MAX_UPDATE_IMAGE_BYTES: u64 = 576 * 1024 * 1024;

#[cfg(test)]
mod tests;
