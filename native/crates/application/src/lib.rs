#![forbid(unsafe_code)]

//! Strict application-package validation.
//!
//! This crate validates the documented local application manifest, enforces
//! package containment, verifies content with a built-in SHA-256 implementation,
//! and returns bounded plain text for the native Windows surface. It does
//! not verify a publisher, launch a process, or execute application code.

mod authoring;
mod installed;
mod manifest;
mod network_policy;
mod package;
pub mod sha256;

use std::{fmt, io};

pub use authoring::write_text_package;
pub use installed::{InstalledApplication, InstalledApplicationError};
pub use manifest::{
    ApplicationIdentity, ApplicationManifest, is_valid_application_id, is_valid_display_name,
};
pub use package::{ApplicationPackage, VerifiedContent, validate_text_content};

/// Maximum manifest size accepted before UTF-8 or JSON decoding.
pub const MAX_MANIFEST_BYTES: usize = 16 * 1024;

/// Maximum raw text-content size accepted before UTF-8 decoding.
pub const MAX_CONTENT_BYTES: usize = 8 * 1024;

/// Maximum installed application-record size accepted before UTF-8 or JSON
/// decoding.
pub const MAX_INSTALL_RECORD_BYTES: usize = 16 * 1024;

/// Maximum executable size checked by the installed-record foundation.
pub const MAX_EXECUTABLE_BYTES: usize = 128 * 1024 * 1024;

/// The only content format supported by application package version 1.0.
pub const TEXT_CONTENT_FORMAT: &str = "anodrel.text.v1";

#[derive(Debug)]
pub enum ApplicationError {
    PackageDestinationExists,
    InvalidPackageDestination,
    Io(io::Error),
    ManifestTooLarge,
    ContentTooLarge,
    InvalidManifest,
    InvalidContentPath,
    ContentOutsidePackage,
    ContentDigestMismatch,
    InvalidText,
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackageDestinationExists => {
                formatter.write_str("application package destination already exists")
            }
            Self::InvalidPackageDestination => {
                formatter.write_str("application package destination is invalid")
            }
            Self::Io(error) => write!(formatter, "application package I/O failed: {error}"),
            Self::ManifestTooLarge => write!(formatter, "application manifest exceeds its limit"),
            Self::ContentTooLarge => write!(formatter, "application content exceeds its limit"),
            Self::InvalidManifest => write!(formatter, "application manifest is invalid"),
            Self::InvalidContentPath => write!(formatter, "application content path is invalid"),
            Self::ContentOutsidePackage => {
                write!(
                    formatter,
                    "application content resolves outside its package"
                )
            }
            Self::ContentDigestMismatch => {
                write!(formatter, "application content digest does not match")
            }
            Self::InvalidText => write!(formatter, "application text content is invalid"),
        }
    }
}

impl std::error::Error for ApplicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for ApplicationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
