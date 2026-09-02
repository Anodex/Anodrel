//! Catalogue-specific bounded composition above direct CMS primitives.

use anodrel_update_catalogue::{MAX_UPDATE_CATALOGUE_BYTES, UpdateCatalogue};
use anodrel_windows_signing::{sign_attached_message, verify_attached_message};

use crate::{MAX_SIGNED_UPDATE_CATALOGUE_BYTES, UpdateCatalogueSignatureError};

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
/// The returned portable value is usable only as an input to later explicit
/// installed-identity/version, retrieval, and signed-installer update gates.
pub fn verify_update_catalogue(
    signed_catalogue: &[u8],
    expected_fingerprint: [u8; 32],
) -> Result<UpdateCatalogue, UpdateCatalogueSignatureError> {
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
    UpdateCatalogue::parse(text).map_err(UpdateCatalogueSignatureError::CatalogueInvalid)
}
