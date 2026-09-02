//! Closed errors for the opaque host-owned Windows update flow.

use std::fmt;

use anodrel_windows_update_cache::UpdateCacheError;
use anodrel_windows_update_download::{
    UpdateCatalogueDiscoveryError, UpdateDownloadError, UpdateImageAcceptanceError,
};
use anodrel_windows_update_handoff::UpdateHandoffError;

/// A native host could not discover one safe update offer.
#[derive(Debug)]
pub enum UpdateOfferError {
    /// Fixed cache selection or constrained cache recovery did not complete.
    CacheInvalid(UpdateCacheError),
    /// Signed installed policy did not yield one safe current candidate.
    DiscoveryInvalid(UpdateCatalogueDiscoveryError),
}

impl fmt::Display for UpdateOfferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CacheInvalid(_) => "the private update cache is unavailable",
            Self::DiscoveryInvalid(_) => "a signed update could not be discovered",
        })
    }
}

impl std::error::Error for UpdateOfferError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CacheInvalid(error) => Some(error),
            Self::DiscoveryInvalid(error) => Some(error),
        }
    }
}

/// A discovered update could not become an opaque UAC-ready image.
#[derive(Debug)]
pub enum UpdateImagePreparationError {
    /// The private bounded image transfer could not complete.
    DownloadInvalid(UpdateDownloadError),
    /// The downloaded image did not become the locked signed candidate.
    ImageInvalid(UpdateImageAcceptanceError),
}

impl fmt::Display for UpdateImagePreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DownloadInvalid(_) => "the signed update image could not be downloaded",
            Self::ImageInvalid(_) => "the signed update image could not be accepted",
        })
    }
}

impl std::error::Error for UpdateImagePreparationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DownloadInvalid(error) => Some(error),
            Self::ImageInvalid(error) => Some(error),
        }
    }
}

/// A UAC-ready update could not be handed to Windows.
#[derive(Debug)]
pub enum UpdateLaunchError {
    /// Windows did not start or expose the fixed elevated update process.
    HandoffInvalid(UpdateHandoffError),
}

impl fmt::Display for UpdateLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the signed update could not be handed to Windows")
    }
}

impl std::error::Error for UpdateLaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::HandoffInvalid(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{UpdateImagePreparationError, UpdateLaunchError, UpdateOfferError};

    #[test]
    fn flow_error_messages_do_not_expose_paths_or_native_statuses() {
        assert_eq!(
            UpdateOfferError::CacheInvalid(
                anodrel_windows_update_cache::UpdateCacheError::DirectoryInvalid
            )
            .to_string(),
            "the private update cache is unavailable"
        );
        assert_eq!(
            UpdateImagePreparationError::ImageInvalid(
                anodrel_windows_update_download::UpdateImageAcceptanceError::CandidateMismatch
            )
            .to_string(),
            "the signed update image could not be accepted"
        );
        assert_eq!(
            UpdateLaunchError::HandoffInvalid(
                anodrel_windows_update_handoff::UpdateHandoffError::UserDeclined
            )
            .to_string(),
            "the signed update could not be handed to Windows"
        );
    }
}
