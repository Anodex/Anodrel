//! Closed errors for signed installed-policy catalogue discovery.

use std::fmt;

use anodrel_windows_http::WindowsHttpsError;
use anodrel_windows_policy::PolicyStoreError;
use anodrel_windows_signature::SignatureError;
use anodrel_windows_update_catalogue_signature::UpdateCatalogueSignatureError;

use crate::UpdateDownloadError;

/// A signed installed release could not yield one preflight-eligible update.
#[derive(Debug)]
pub enum UpdateCatalogueDiscoveryError {
    /// Fixed machine policy could not select a valid installed application.
    InstalledPolicyInvalid(PolicyStoreError),
    /// Windows could not verify the installed executable's Authenticode signer.
    InstalledSignatureInvalid(SignatureError),
    /// The installed executable signer differed from the approved machine record.
    InstalledPublisherMismatch,
    /// The installed release did not opt into an update catalogue location.
    CatalogueSourceUnavailable,
    /// The bounded direct HTTPS catalogue transfer could not meet its contract.
    RetrievalFailed(WindowsHttpsError),
    /// The retrieved attached CMS catalogue did not verify for the installed signer.
    CatalogueInvalid(UpdateCatalogueSignatureError),
    /// The verified catalogue did not remain eligible against current installed facts.
    CandidateInvalid(UpdateDownloadError),
}

impl fmt::Display for UpdateCatalogueDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InstalledPolicyInvalid(_) => "the installed application policy is invalid",
            Self::InstalledSignatureInvalid(_) => {
                "Windows did not accept the installed executable signature"
            }
            Self::InstalledPublisherMismatch => {
                "the installed executable publisher does not match policy"
            }
            Self::CatalogueSourceUnavailable => "this installed release has no update source",
            Self::RetrievalFailed(_) => "the update catalogue transfer could not complete",
            Self::CatalogueInvalid(_) => "the update catalogue is invalid",
            Self::CandidateInvalid(_) => "the update catalogue is not eligible",
        })
    }
}

impl std::error::Error for UpdateCatalogueDiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstalledPolicyInvalid(error) => Some(error),
            Self::InstalledSignatureInvalid(error) => Some(error),
            Self::RetrievalFailed(error) => Some(error),
            Self::CatalogueInvalid(error) => Some(error),
            Self::CandidateInvalid(error) => Some(error),
            Self::InstalledPublisherMismatch | Self::CatalogueSourceUnavailable => None,
        }
    }
}
