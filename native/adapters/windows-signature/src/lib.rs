#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows Authenticode verification.
//!
//! This adapter is a narrow verification primitive. It validates an embedded
//! Authenticode signature through Windows policy and returns only the
//! SHA-256 fingerprint of the leaf signing certificate. It does not launch a
//! process, establish package trust, or expose native trust diagnostics.

mod raw;

use std::{fmt, path::Path};

/// A fixed SHA-256 fingerprint for a trusted leaf signing certificate.
///
/// This value is meant for host policy comparison. It intentionally does not
/// implement display formatting because a future renderer must not become a
/// certificate-diagnostics surface.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct SignerFingerprint([u8; 32]);

impl SignerFingerprint {
    /// Returns the fixed-width fingerprint bytes for host policy comparison.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for SignerFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SignerFingerprint(..)")
    }
}

/// A safe category for an executable-signature verification failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureError {
    /// The caller did not supply an absolute Windows path.
    InvalidPath,
    /// Windows did not accept the executable's embedded signature.
    TrustRejected,
    /// Windows did not make a provider state available after verification.
    ProviderStateUnavailable,
    /// The accepted signature did not expose a leaf signer.
    SignerUnavailable,
    /// The leaf signer did not expose a certificate context.
    CertificateUnavailable,
    /// Windows did not provide a SHA-256 fingerprint for the leaf certificate.
    FingerprintUnavailable,
}

impl fmt::Display for SignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidPath => "the executable path is invalid",
            Self::TrustRejected => "Windows did not accept the executable signature",
            Self::ProviderStateUnavailable => "Windows trust state is unavailable",
            Self::SignerUnavailable => "the accepted signature has no leaf signer",
            Self::CertificateUnavailable => "the leaf signer certificate is unavailable",
            Self::FingerprintUnavailable => "the leaf signer fingerprint is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SignatureError {}

/// Verifies an embedded Authenticode signature and returns its leaf fingerprint.
///
/// The caller owns path containment and must pass an absolute canonical
/// executable path held entirely inside host policy. No raw Windows status,
/// certificate name, path, or signature material is returned on failure.
pub fn verify_embedded_signature(path: &Path) -> Result<SignerFingerprint, SignatureError> {
    if !path.is_absolute() {
        return Err(SignatureError::InvalidPath);
    }
    raw::verify_embedded_signature(path).map(SignerFingerprint)
}

#[cfg(test)]
mod tests {
    use super::{SignatureError, SignerFingerprint, verify_embedded_signature};
    use std::path::Path;

    #[test]
    fn relative_paths_are_rejected_before_windows_trust_is_called() {
        assert_eq!(
            verify_embedded_signature(Path::new("bin/client.exe")),
            Err(SignatureError::InvalidPath)
        );
    }

    #[test]
    fn a_fingerprint_is_opaque_in_debug_output() {
        let fingerprint = SignerFingerprint([0xA5; 32]);
        assert_eq!(format!("{fingerprint:?}"), "SignerFingerprint(..)");
    }

    #[test]
    fn an_unsigned_current_test_binary_is_rejected() {
        let binary = std::env::current_exe().expect("test executable path is available");
        assert_eq!(
            verify_embedded_signature(&binary),
            Err(SignatureError::TrustRejected)
        );
    }

    #[test]
    #[ignore = "requires ANODREL_SIGNED_TEST_BINARY to name an embedded-signed executable"]
    fn an_operator_supplied_embedded_signed_binary_yields_a_nonzero_fingerprint() {
        let binary = std::env::var_os("ANODREL_SIGNED_TEST_BINARY")
            .map(std::path::PathBuf::from)
            .expect("ANODREL_SIGNED_TEST_BINARY names a signed executable");
        let fingerprint =
            verify_embedded_signature(&binary).expect("Windows accepts the embedded signature");
        assert!(fingerprint.as_bytes().iter().any(|byte| *byte != 0));
    }
}
