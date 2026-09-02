//! Closed safe failures from the fixed development update acceptance runner.

use std::fmt;

use anodrel_windows_update_consent::UpdateConsentError;
use anodrel_windows_update_handoff::UpdateHandoffError;
use anodrel_windows_updater::{
    UpdateCompletionError, UpdateImagePreparationError, UpdateLaunchError, UpdateOfferError,
};

/// The fixed acceptance diagnostic could not verify a product update.
#[derive(Debug)]
pub enum ProductUpdateAcceptanceError {
    /// Signed policy did not produce one eligible update offer.
    Offer(UpdateOfferError),
    /// The native Anodrel confirmation could not be displayed.
    Consent(UpdateConsentError),
    /// The private installer image could not be prepared.
    Preparation(UpdateImagePreparationError),
    /// Windows could not begin the fixed elevated update command.
    Launch(UpdateLaunchError),
    /// The elevated update process could not be observed safely.
    Observation(UpdateHandoffError),
    /// The installed policy did not prove the selected candidate afterwards.
    Postcondition(UpdateCompletionError),
}

impl fmt::Display for ProductUpdateAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Offer(_) => "the fixed fixture update could not be discovered",
            Self::Consent(_) => "the fixed fixture update confirmation could not be displayed",
            Self::Preparation(_) => "the fixed fixture update image could not be prepared",
            Self::Launch(_) => "the fixed fixture update could not be handed to Windows",
            Self::Observation(_) => "the fixed fixture update could not be observed",
            Self::Postcondition(_) => {
                "the fixed fixture update was not verified after installation"
            }
        })
    }
}

impl std::error::Error for ProductUpdateAcceptanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Offer(error) => Some(error),
            Self::Consent(error) => Some(error),
            Self::Preparation(error) => Some(error),
            Self::Launch(error) => Some(error),
            Self::Observation(error) => Some(error),
            Self::Postcondition(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProductUpdateAcceptanceError;

    #[test]
    fn diagnostic_failures_do_not_expose_paths_or_native_status() {
        let error = ProductUpdateAcceptanceError::Offer(
            anodrel_windows_updater::UpdateOfferError::CacheInvalid(
                anodrel_windows_update_cache::UpdateCacheError::DirectoryInvalid,
            ),
        );
        assert_eq!(
            error.to_string(),
            "the fixed fixture update could not be discovered"
        );
        assert!(!error.to_string().contains(":\\"));
    }
}
