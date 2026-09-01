//! Closed failure categories for release-bundle authoring.

use std::fmt;

use anodrel_release_bundle::ReleaseBundleError;

/// A release bundle could not be authored within the fixed boundary.
#[derive(Debug)]
pub enum BundleAuthorError {
    /// The source path was not one existing absolute normal directory.
    SourceInvalid,
    /// An entry beneath the source was a link, special entry, or unreadable name.
    SourceEntryInvalid,
    /// The source directory could not be enumerated or a regular file could not be read.
    SourceReadFailed,
    /// The source could not fit inside the version-1 bundle limits.
    SourceLimitExceeded,
    /// The output path was not a safe fresh absolute file path outside the source tree.
    OutputInvalid,
    /// The output already existed and must remain unchanged.
    OutputAlreadyExists,
    /// The fresh output file could not be created.
    OutputCreationFailed,
    /// The one new output file could not receive all bundle bytes.
    OutputWriteFailed,
    /// The one new output file could not be synchronized.
    OutputSyncFailed,
    /// The owned bundle encoder or parser rejected the assembled entries.
    BundleInvalid(ReleaseBundleError),
}

impl fmt::Display for BundleAuthorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SourceInvalid => "the release bundle source directory is invalid",
            Self::SourceEntryInvalid => "the release bundle source contains an invalid entry",
            Self::SourceReadFailed => "the release bundle source could not be read",
            Self::SourceLimitExceeded => "the release bundle source exceeds version-1 limits",
            Self::OutputInvalid => "the release bundle output path is invalid",
            Self::OutputAlreadyExists => "the release bundle output already exists",
            Self::OutputCreationFailed => "the release bundle output could not be created",
            Self::OutputWriteFailed => "the release bundle output could not be written",
            Self::OutputSyncFailed => "the release bundle output could not be synchronized",
            Self::BundleInvalid(_) => "the release bundle could not be encoded",
        })
    }
}

impl std::error::Error for BundleAuthorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BundleInvalid(error) => Some(error),
            Self::SourceInvalid
            | Self::SourceEntryInvalid
            | Self::SourceReadFailed
            | Self::SourceLimitExceeded
            | Self::OutputInvalid
            | Self::OutputAlreadyExists
            | Self::OutputCreationFailed
            | Self::OutputWriteFailed
            | Self::OutputSyncFailed => None,
        }
    }
}
