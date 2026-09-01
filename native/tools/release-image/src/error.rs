//! Safe release-image assembly failure categories.

use std::fmt;

use anodrel_windows_installer::{ReleaseManifestError, ReleasePayloadError};

/// A release image could not be assembled or verified safely.
#[derive(Debug)]
pub enum ReleaseImageError {
    /// An inspected release image was not one readable absolute regular file.
    ImageInvalid,
    /// The template path was not one readable absolute regular file.
    TemplateInvalid,
    /// The requested output path was unsafe or did not have an existing parent.
    OutputInvalid,
    /// The output already existed and must not be overwritten.
    OutputAlreadyExists,
    /// The source template could not be copied to the new output path.
    CopyFailed,
    /// Manifest bytes were not UTF-8 or did not meet their strict contract.
    ManifestInvalid(ReleaseManifestError),
    /// Bundle bytes did not match the manifest or bundle contract.
    PayloadInvalid(ReleasePayloadError),
    /// The inspected release image named a different publisher than expected.
    PublisherMismatch,
    /// Windows could not begin a resource update transaction for the output.
    ResourceTransactionUnavailable,
    /// Windows rejected one fixed release resource update.
    ResourceWriteFailed,
    /// Windows could not commit the resource update transaction.
    ResourceCommitFailed,
    /// The committed output did not expose the exact expected resource bytes.
    ResourceVerificationFailed,
}

impl fmt::Display for ReleaseImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ImageInvalid => "the release image is invalid",
            Self::TemplateInvalid => "the installer template is invalid",
            Self::OutputInvalid => "the release image output path is invalid",
            Self::OutputAlreadyExists => "the release image output already exists",
            Self::CopyFailed => "the installer template could not be copied",
            Self::ManifestInvalid(_) => "the release manifest is invalid",
            Self::PayloadInvalid(_) => "the release payload is invalid",
            Self::PublisherMismatch => "the release image publisher does not match",
            Self::ResourceTransactionUnavailable => {
                "Windows could not begin a release resource update"
            }
            Self::ResourceWriteFailed => "Windows could not write a release resource",
            Self::ResourceCommitFailed => "Windows could not commit release resources",
            Self::ResourceVerificationFailed => "the release image resources did not verify",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ReleaseImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ManifestInvalid(error) => Some(error),
            Self::PayloadInvalid(error) => Some(error),
            Self::ImageInvalid
            | Self::TemplateInvalid
            | Self::OutputInvalid
            | Self::OutputAlreadyExists
            | Self::CopyFailed
            | Self::PublisherMismatch
            | Self::ResourceTransactionUnavailable
            | Self::ResourceWriteFailed
            | Self::ResourceCommitFailed
            | Self::ResourceVerificationFailed => None,
        }
    }
}
