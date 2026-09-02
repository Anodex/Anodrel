//! Catalogue-specific bounded composition above direct CMS primitives.

use anodrel_update_catalogue::{MAX_UPDATE_CATALOGUE_BYTES, UpdateCatalogue, UpdateInstaller};
use anodrel_windows_installer::{PackageVersion, ReleaseManifest};
use anodrel_windows_signing::{sign_attached_message, verify_attached_message};

use crate::{MAX_SIGNED_UPDATE_CATALOGUE_BYTES, UpdateCatalogueSignatureError};

/// One catalogue that passed attached-CMS verification for an exact publisher.
///
/// This value can be created only by [`verify_update_catalogue`]. It proves the
/// source catalogue's bounded CMS signature, but callers must still compare its
/// identity and version to installed facts before offering an update.
pub struct VerifiedUpdateCatalogue {
    catalogue: UpdateCatalogue,
}

impl VerifiedUpdateCatalogue {
    /// Returns the verified catalogue's declared application identity.
    #[must_use]
    pub fn application_id(&self) -> &str {
        self.catalogue.application_id()
    }

    /// Returns the verified catalogue's declared release version.
    #[must_use]
    pub const fn package_version(&self) -> PackageVersion {
        self.catalogue.package_version()
    }

    /// Compares the verified catalogue with host-held installed facts.
    #[must_use]
    pub fn matches_installed(&self, application_id: &str, publisher: [u8; 32]) -> bool {
        self.catalogue.matches_installed(application_id, publisher)
    }

    /// Compares the verified catalogue with one locked installer release.
    ///
    /// The caller must establish the image's Windows Authenticode acceptance
    /// independently before using this exact data comparison.
    #[must_use]
    pub fn matches_release(&self, release: &ReleaseManifest) -> bool {
        self.catalogue.matches_release(release)
    }

    /// Returns whether the verified catalogue is newer than one installed release.
    #[must_use]
    pub fn is_newer_than(&self, installed: PackageVersion) -> bool {
        self.catalogue.is_newer_than(installed)
    }

    /// Returns the verified catalogue's exact installer retrieval contract.
    #[must_use]
    pub fn installer(&self) -> &UpdateInstaller {
        self.catalogue.installer()
    }
}

impl std::fmt::Debug for VerifiedUpdateCatalogue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("VerifiedUpdateCatalogue")
            .field(&self.catalogue)
            .finish()
    }
}

/// Signs exact valid catalogue UTF-8 bytes with one selected current-user key.
///
/// The returned attached CMS envelope is independently verified against the
/// same fingerprint before success, so a successful result is one bounded
/// single-signer envelope whose decoded bytes re-parse as the original strict
/// catalogue. No certificate chain trust or timestamp is claimed.
pub fn sign_update_catalogue(
    catalogue: &str,
    fingerprint: [u8; 32],
) -> Result<Vec<u8>, UpdateCatalogueSignatureError> {
    UpdateCatalogue::parse(catalogue).map_err(UpdateCatalogueSignatureError::CatalogueInvalid)?;
    let signed = sign_attached_message(
        catalogue.as_bytes(),
        MAX_SIGNED_UPDATE_CATALOGUE_BYTES,
        fingerprint,
    )
    .map_err(UpdateCatalogueSignatureError::SignatureInvalid)?;
    let _ = verify_update_catalogue(&signed, fingerprint)?;
    Ok(signed)
}

/// Verifies a bounded attached CMS catalogue against one exact publisher.
///
/// The returned value is usable only as an input to later explicit
/// installed-identity/version, retrieval, and signed-installer update gates.
pub fn verify_update_catalogue(
    signed_catalogue: &[u8],
    expected_fingerprint: [u8; 32],
) -> Result<VerifiedUpdateCatalogue, UpdateCatalogueSignatureError> {
    if signed_catalogue.len() > MAX_SIGNED_UPDATE_CATALOGUE_BYTES {
        return Err(UpdateCatalogueSignatureError::SignatureInvalid(
            anodrel_windows_signing::WindowsSigningError::MessageLimitInvalid,
        ));
    }
    let decoded = verify_attached_message(
        signed_catalogue,
        MAX_UPDATE_CATALOGUE_BYTES,
        expected_fingerprint,
    )
    .map_err(UpdateCatalogueSignatureError::SignatureInvalid)?;
    let text = std::str::from_utf8(&decoded).map_err(|_| {
        UpdateCatalogueSignatureError::CatalogueInvalid(
            anodrel_update_catalogue::UpdateCatalogueError::Invalid,
        )
    })?;
    let catalogue =
        UpdateCatalogue::parse(text).map_err(UpdateCatalogueSignatureError::CatalogueInvalid)?;
    Ok(VerifiedUpdateCatalogue { catalogue })
}
