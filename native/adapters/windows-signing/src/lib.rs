#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Direct Windows signing primitives for Anodrel-owned release tooling.
//!
//! This adapter selects one explicit certificate from the current user's `MY`
//! store. It signs an Authenticode file or an attached CMS message and verifies
//! a single attached CMS message against an exact certificate fingerprint. It
//! does not select a certificate authority, import a key, create trust, access
//! a network, install a release, or launch a process.

mod authenticode;
mod certificate;
mod error;
mod message;

use std::path::Path;

pub use error::WindowsSigningError;

/// Signs one existing file in place through Windows Authenticode.
///
/// Callers must independently ensure the file is an owned fresh release image.
/// This function neither copies nor verifies the file before signing.
pub fn sign_authenticode_file(
    path: &Path,
    fingerprint: [u8; 32],
) -> Result<(), WindowsSigningError> {
    authenticode::sign_file(path, fingerprint)
}

/// Creates one attached CMS signature with an exact current-user certificate.
///
/// The message and maximum output length are caller-selected bounded inputs.
/// The returned CMS envelope contains the original message and one signer
/// certificate. It has no timestamp, network operation, or file operation.
pub fn sign_attached_message(
    message: &[u8],
    maximum_output_bytes: usize,
    fingerprint: [u8; 32],
) -> Result<Vec<u8>, WindowsSigningError> {
    message::sign_attached(message, maximum_output_bytes, fingerprint)
}

/// Verifies one single-signer attached CMS message against an exact publisher.
///
/// The signed-envelope and decoded-message size limits are both supplied by
/// the caller. The returned bytes are cryptographically authenticated but do
/// not imply certificate-chain trust, timestamp validity, or installer trust.
pub fn verify_attached_message(
    signed_message: &[u8],
    maximum_decoded_bytes: usize,
    expected_fingerprint: [u8; 32],
) -> Result<Vec<u8>, WindowsSigningError> {
    message::verify_attached(signed_message, maximum_decoded_bytes, expected_fingerprint)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        WindowsSigningError, sign_attached_message, sign_authenticode_file, verify_attached_message,
    };

    #[test]
    fn caller_bounds_fail_before_certificate_or_message_access() {
        assert_eq!(
            sign_attached_message(b"message", 0, [0; 32]),
            Err(WindowsSigningError::MessageLimitInvalid)
        );
        assert_eq!(
            verify_attached_message(b"message", 0, [0; 32]),
            Err(WindowsSigningError::MessageLimitInvalid)
        );
    }

    #[test]
    fn relative_authenticode_paths_fail_before_certificate_selection() {
        assert_eq!(
            sign_authenticode_file(Path::new("release.exe"), [0; 32]),
            Err(WindowsSigningError::AuthenticodePathInvalid)
        );
    }
}
