//! Closed failure categories for owned Windows release signing.

use std::fmt;

use anodrel_release_image::ReleaseImageError;
use anodrel_windows_signature::SignatureError;

/// A release image could not become one verified owned signed output.
#[derive(Debug)]
pub enum ReleaseSignError {
    /// The input did not contain one valid absolute Anodrel release image.
    InputInvalid(ReleaseImageError),
    /// The output path was not an absolute fresh path with a normal parent.
    OutputInvalid,
    /// The output already existed and must remain unchanged.
    OutputAlreadyExists,
    /// The checked input could not be copied to the one new output path.
    CopyFailed,
    /// The input image was too large for the bounded release signing operation.
    InputTooLarge,
    /// The supplied certificate fingerprint was not lowercase SHA-256 text.
    CertificateFingerprintInvalid,
    /// Windows could not open the current-user certificate store.
    CertificateStoreUnavailable,
    /// The current-user store did not contain the exact requested certificate.
    CertificateUnavailable,
    /// Windows could not load its direct Authenticode signing entry points.
    SigningUnavailable,
    /// Windows did not produce an Authenticode signature for the fresh copy.
    SigningFailed,
    /// The fresh signed output no longer contained the checked release image.
    SignedImageInvalid(ReleaseImageError),
    /// Windows did not accept the fresh output's Authenticode signature.
    OutputSignatureInvalid(SignatureError),
    /// The accepted output signer was not the exact selected certificate.
    OutputPublisherMismatch,
}

impl fmt::Display for ReleaseSignError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputInvalid(_) => "the unsigned release image is invalid",
            Self::OutputInvalid => "the signed release image output path is invalid",
            Self::OutputAlreadyExists => "the signed release image output already exists",
            Self::CopyFailed => "the unsigned release image could not be copied",
            Self::InputTooLarge => "the unsigned release image exceeds the signing limit",
            Self::CertificateFingerprintInvalid => "the signing certificate fingerprint is invalid",
            Self::CertificateStoreUnavailable => "the current-user signing store is unavailable",
            Self::CertificateUnavailable => "the selected signing certificate is unavailable",
            Self::SigningUnavailable => "Windows signing is unavailable",
            Self::SigningFailed => "Windows could not sign the release image",
            Self::SignedImageInvalid(_) => "the signed release image is invalid",
            Self::OutputSignatureInvalid(_) => "Windows did not accept the signed release image",
            Self::OutputPublisherMismatch => {
                "the signed release image used an unexpected publisher"
            }
        })
    }
}

impl std::error::Error for ReleaseSignError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InputInvalid(error) | Self::SignedImageInvalid(error) => Some(error),
            Self::OutputSignatureInvalid(error) => Some(error),
            Self::OutputInvalid
            | Self::OutputAlreadyExists
            | Self::CopyFailed
            | Self::InputTooLarge
            | Self::CertificateFingerprintInvalid
            | Self::CertificateStoreUnavailable
            | Self::CertificateUnavailable
            | Self::SigningUnavailable
            | Self::SigningFailed
            | Self::OutputPublisherMismatch => None,
        }
    }
}
