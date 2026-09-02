//! Boundary checks that do not require an operator certificate.

use anodrel_windows_signing::WindowsSigningError;

use crate::{UpdateCatalogueSignatureError, sign_update_catalogue, verify_update_catalogue};

#[test]
fn invalid_catalogue_cannot_attempt_certificate_signing() {
    assert!(matches!(
        sign_update_catalogue("not JSON", [0; 32]),
        Err(UpdateCatalogueSignatureError::CatalogueInvalid(_))
    ));
}

#[test]
fn invalid_cms_bytes_cannot_be_treated_as_a_catalogue() {
    assert!(matches!(
        verify_update_catalogue(b"not a CMS message", [0; 32]),
        Err(UpdateCatalogueSignatureError::SignatureInvalid(
            WindowsSigningError::MessageVerificationFailed
        ))
    ));
}
