//! Safe release-bundle failure categories.

use std::fmt;

/// A release bundle did not meet its bounded binary contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseBundleError {
    /// The complete bundle exceeds its fixed size bound.
    TooLarge,
    /// The bytes do not begin with the required bundle header.
    HeaderInvalid,
    /// The bundle uses an unsupported format version.
    VersionUnsupported,
    /// The declared entry count exceeds the fixed bound.
    EntryCountInvalid,
    /// An entry ended before its declared fields or bytes did.
    Truncated,
    /// An entry path was malformed or exceeded its fixed bound.
    PathInvalid,
    /// Entry paths were not strictly ascending.
    EntryOrderInvalid,
    /// An entry's raw bytes did not match its declared SHA-256 digest.
    DigestMismatch,
    /// Bytes remained after the final declared entry.
    TrailingData,
}

impl fmt::Display for ReleaseBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooLarge => "release bundle exceeds its limit",
            Self::HeaderInvalid => "release bundle header is invalid",
            Self::VersionUnsupported => "release bundle version is unsupported",
            Self::EntryCountInvalid => "release bundle entry count is invalid",
            Self::Truncated => "release bundle is truncated",
            Self::PathInvalid => "release bundle entry path is invalid",
            Self::EntryOrderInvalid => "release bundle entry order is invalid",
            Self::DigestMismatch => "release bundle entry digest does not match",
            Self::TrailingData => "release bundle has trailing data",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ReleaseBundleError {}
