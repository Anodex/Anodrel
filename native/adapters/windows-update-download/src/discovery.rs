//! Retrieval of one attached CMS catalogue from signed installed policy.

use anodrel_windows_http::get_https;
use anodrel_windows_policy::load_installed_application;
use anodrel_windows_signature::verify_embedded_signature;
use anodrel_windows_update_catalogue_signature::{
    MAX_SIGNED_UPDATE_CATALOGUE_BYTES, verify_update_catalogue,
};

use crate::{
    PreparedUpdateDownload, UpdateCatalogueDiscoveryError, prepare_current_update_download,
};

/// Retrieves and preflights one update from the signed installed policy source.
///
/// `application_id` must be selected by native updater composition, never an
/// application, protocol message, command line, environment, or UI. The
/// selected record must opt into a strict catalogue source and retain a valid
/// installed executable signature before this function makes one transfer.
pub fn retrieve_current_update_download(
    application_id: &str,
) -> Result<PreparedUpdateDownload, UpdateCatalogueDiscoveryError> {
    let installed = load_installed_application(application_id)
        .map_err(UpdateCatalogueDiscoveryError::InstalledPolicyInvalid)?;
    let signer = verify_embedded_signature(installed.executable_path())
        .map_err(UpdateCatalogueDiscoveryError::InstalledSignatureInvalid)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(UpdateCatalogueDiscoveryError::InstalledPublisherMismatch);
    }
    let location = installed
        .update_catalogue_location()
        .ok_or(UpdateCatalogueDiscoveryError::CatalogueSourceUnavailable)?;
    let mut envelope = Vec::with_capacity(4 * 1024);
    get_https(
        location.origin(),
        location.request_path(),
        Some(200),
        MAX_SIGNED_UPDATE_CATALOGUE_BYTES,
        &mut |chunk| {
            envelope.try_reserve(chunk.len()).map_err(|_| ())?;
            envelope.extend_from_slice(chunk);
            Ok(())
        },
    )
    .map_err(UpdateCatalogueDiscoveryError::RetrievalFailed)?;
    let catalogue = verify_update_catalogue(&envelope, signer.as_bytes())
        .map_err(UpdateCatalogueDiscoveryError::CatalogueInvalid)?;
    prepare_current_update_download(catalogue)
        .map_err(UpdateCatalogueDiscoveryError::CandidateInvalid)
}

#[cfg(test)]
mod tests {
    use anodrel_windows_policy::PolicyStoreError;

    use super::retrieve_current_update_download;
    use crate::UpdateCatalogueDiscoveryError;

    #[test]
    fn invalid_host_identity_fails_before_registry_or_network_access() {
        assert!(matches!(
            retrieve_current_update_download("org.anodrel/escape"),
            Err(UpdateCatalogueDiscoveryError::InstalledPolicyInvalid(
                PolicyStoreError::InvalidApplicationId
            ))
        ));
    }
}
