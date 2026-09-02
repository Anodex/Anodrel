//! Ordered opaque composition of private update discovery through UAC handoff.

use std::fmt;

use anodrel_windows_installer::PackageVersion;
use anodrel_windows_update_cache::{UpdateCache, open_current_update_cache, recover_update_cache};
use anodrel_windows_update_download::{
    PreparedUpdateDownload, VerifiedDownloadedInstaller, download_prepared_update,
    retrieve_current_update_download, verify_downloaded_update_image,
};
use anodrel_windows_update_handoff::{ElevatedUpdateProcess, begin_elevated_update};

use crate::{UpdateImagePreparationError, UpdateLaunchError, UpdateOfferError};

/// One signed current update candidate held with its fixed private cache.
pub struct AvailableUpdate {
    cache: UpdateCache,
    candidate: PreparedUpdateDownload,
}

impl fmt::Debug for AvailableUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AvailableUpdate(..)")
    }
}

/// One locked exact update installer ready only for the UAC handoff.
pub struct ReadyUpdate {
    image: VerifiedDownloadedInstaller,
}

impl fmt::Debug for ReadyUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReadyUpdate(..)")
    }
}

impl AvailableUpdate {
    /// Returns the signed candidate version for an Anodrel-owned host prompt.
    #[must_use]
    pub const fn candidate_version(&self) -> PackageVersion {
        self.candidate.candidate_version()
    }

    /// Downloads and locks this exact discovered candidate into its fixed cache.
    ///
    /// A host must place this mutating transfer behind its own explicit user
    /// decision. This does not elevate, launch, restart, or install anything.
    pub fn download(self) -> Result<ReadyUpdate, UpdateImagePreparationError> {
        let downloaded = download_prepared_update(&self.candidate, self.cache.directory())
            .map_err(UpdateImagePreparationError::DownloadInvalid)?;
        let image = verify_downloaded_update_image(&self.candidate, downloaded)
            .map_err(UpdateImagePreparationError::ImageInvalid)?;
        Ok(ReadyUpdate { image })
    }
}

impl ReadyUpdate {
    /// Requests Windows UAC for the fixed `runas update` handoff only.
    ///
    /// A host must place this behind explicit user consent. Windows can return
    /// a normal cancellation outcome; a successful handoff is not installation
    /// proof and its process must be waited away from a UI thread.
    pub fn begin_elevation(self) -> Result<ElevatedUpdateProcess, UpdateLaunchError> {
        begin_elevated_update(self.image).map_err(UpdateLaunchError::HandoffInvalid)
    }
}

/// Discovers one signed current candidate through the only supported ordering.
///
/// `application_id` must be selected by native host composition, never by an
/// application, protocol message, command line, environment value, or UI. The
/// fixed cache is recovered before one signed-policy catalogue request. This
/// does not download an image, request UAC, launch a process, or install.
pub fn discover_current_update(application_id: &str) -> Result<AvailableUpdate, UpdateOfferError> {
    let cache =
        open_current_update_cache(application_id).map_err(UpdateOfferError::CacheInvalid)?;
    recover_update_cache(&cache).map_err(UpdateOfferError::CacheInvalid)?;
    let candidate = retrieve_current_update_download(application_id)
        .map_err(UpdateOfferError::DiscoveryInvalid)?;
    Ok(AvailableUpdate { cache, candidate })
}

#[cfg(test)]
mod tests {
    use anodrel_windows_policy::PolicyStoreError;

    use super::discover_current_update;
    use crate::UpdateOfferError;

    #[test]
    fn invalid_identity_stops_before_cache_recovery_or_network_discovery() {
        assert!(matches!(
            discover_current_update("org.anodrel/escape"),
            Err(UpdateOfferError::CacheInvalid(
                anodrel_windows_update_cache::UpdateCacheError::InstalledPolicyInvalid(
                    PolicyStoreError::InvalidApplicationId
                )
            ))
        ));
    }
}
