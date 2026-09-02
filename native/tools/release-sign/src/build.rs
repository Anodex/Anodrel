//! Checked fresh-output signing composition.

use std::path::Path;

use anodrel_application::sha256;
use anodrel_release_image::verify_release_image_for_publisher;
use anodrel_windows_signature::verify_embedded_signature;
use anodrel_windows_signing::{WindowsSigningError, sign_authenticode_file};

use crate::{ReleaseSignError, output::FreshReleaseImage};

/// Signs one checked release image into one fresh verified output image.
///
/// `input` and `output` must be absolute paths. The input must already carry a
/// valid Anodrel release manifest and bundle that names the same publisher as
/// `certificate_fingerprint`. That fingerprint must name one exact lowercase
/// SHA-256 leaf certificate in the current user's Windows `MY` store. The input
/// is never modified, and the output is removed if copying, signing, release
/// validation, or Windows trust verification fails.
pub fn sign_release_image(
    input: &Path,
    certificate_fingerprint: &str,
    output: &Path,
) -> Result<(), ReleaseSignError> {
    let fingerprint = sha256::parse_lower_hex(certificate_fingerprint)
        .ok_or(ReleaseSignError::CertificateFingerprintInvalid)?;
    verify_release_image_for_publisher(input, fingerprint)
        .map_err(ReleaseSignError::InputInvalid)?;
    let mut output = FreshReleaseImage::copy_from(input, output)?;
    verify_release_image_for_publisher(output.path(), fingerprint)
        .map_err(ReleaseSignError::SignedImageInvalid)?;
    sign_authenticode_file(output.path(), fingerprint).map_err(release_signing_error)?;
    verify_release_image_for_publisher(output.path(), fingerprint)
        .map_err(ReleaseSignError::SignedImageInvalid)?;
    let signer = verify_embedded_signature(output.path())
        .map_err(ReleaseSignError::OutputSignatureInvalid)?;
    if signer.as_bytes() != fingerprint {
        return Err(ReleaseSignError::OutputPublisherMismatch);
    }
    output.keep();
    Ok(())
}

fn release_signing_error(error: WindowsSigningError) -> ReleaseSignError {
    match error {
        WindowsSigningError::CertificateStoreUnavailable => {
            ReleaseSignError::CertificateStoreUnavailable
        }
        WindowsSigningError::CertificateUnavailable => ReleaseSignError::CertificateUnavailable,
        WindowsSigningError::AuthenticodeUnavailable => ReleaseSignError::SigningUnavailable,
        WindowsSigningError::AuthenticodePathInvalid
        | WindowsSigningError::AuthenticodeFailed
        | WindowsSigningError::MessageLimitInvalid
        | WindowsSigningError::MessageSigningFailed
        | WindowsSigningError::MessageVerificationFailed
        | WindowsSigningError::MessageSignerUnavailable
        | WindowsSigningError::MessageFingerprintUnavailable
        | WindowsSigningError::MessagePublisherMismatch => ReleaseSignError::SigningFailed,
    }
}
