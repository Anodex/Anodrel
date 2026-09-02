//! Closed failure categories for verified update-image preparation and staging.

use std::fmt;

use anodrel_windows_http::WindowsHttpsError;
use anodrel_windows_policy::PolicyStoreError;
use anodrel_windows_signature::SignatureError;

/// One verified update catalogue could not become a fresh staged installer.
#[derive(Debug)]
pub enum UpdateDownloadError {
    /// Fixed machine policy could not select a valid installed application.
    InstalledPolicyInvalid(PolicyStoreError),
    /// Windows could not verify the installed executable's Authenticode signer.
    InstalledSignatureInvalid(SignatureError),
    /// The installed executable's signer differed from its machine record.
    InstalledPublisherMismatch,
    /// The selected package root did not use an exact Anodrel release version.
    InstalledVersionInvalid,
    /// The signed catalogue did not match the installed identity and signer.
    CandidateIdentityMismatch,
    /// The signed catalogue was not strictly newer than the installed release.
    CandidateNotNewer,
    /// The updater-selected cache parent was not an existing normal directory.
    CacheParentInvalid,
    /// A previously absent private installer cache file could not be created.
    CacheFileCreationFailed,
    /// A new private installer cache file could not receive one checked chunk.
    CacheFileWriteFailed,
    /// A new private installer cache file could not be synchronized.
    CacheFileSyncFailed,
    /// The bounded direct HTTPS transfer could not meet its contract.
    TransferFailed(WindowsHttpsError),
    /// The staged image's final length or SHA-256 did not match its catalogue.
    ImageMismatch,
}

impl fmt::Display for UpdateDownloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InstalledPolicyInvalid(_) => "the installed application policy is invalid",
            Self::InstalledSignatureInvalid(_) => {
                "Windows did not accept the installed executable signature"
            }
            Self::InstalledPublisherMismatch => {
                "the installed executable publisher does not match policy"
            }
            Self::InstalledVersionInvalid => {
                "the installed package root does not use an Anodrel release version"
            }
            Self::CandidateIdentityMismatch => {
                "the signed update catalogue does not match the installed application"
            }
            Self::CandidateNotNewer => "the signed update is not newer than the installed release",
            Self::CacheParentInvalid => "the updater cache directory is invalid",
            Self::CacheFileCreationFailed => "a private update cache file could not be created",
            Self::CacheFileWriteFailed => "the private update cache file could not be written",
            Self::CacheFileSyncFailed => "the private update cache file could not be synchronized",
            Self::TransferFailed(_) => "the update image transfer could not complete",
            Self::ImageMismatch => "the downloaded update image does not match its catalogue",
        })
    }
}

impl std::error::Error for UpdateDownloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstalledPolicyInvalid(error) => Some(error),
            Self::InstalledSignatureInvalid(error) => Some(error),
            Self::TransferFailed(error) => Some(error),
            Self::InstalledPublisherMismatch
            | Self::InstalledVersionInvalid
            | Self::CandidateIdentityMismatch
            | Self::CandidateNotNewer
            | Self::CacheParentInvalid
            | Self::CacheFileCreationFailed
            | Self::CacheFileWriteFailed
            | Self::CacheFileSyncFailed
            | Self::ImageMismatch => None,
        }
    }
}
