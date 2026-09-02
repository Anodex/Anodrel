//! Closed direct-Windows signing failure categories.

use std::fmt;

/// A direct Windows signing or verification operation could not complete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsSigningError {
    /// The selected current-user certificate store could not be opened.
    CertificateStoreUnavailable,
    /// The current-user store had no exact requested certificate.
    CertificateUnavailable,
    /// Windows could not load its Authenticode signer entry points.
    AuthenticodeUnavailable,
    /// The Authenticode target was not an absolute non-empty Windows path.
    AuthenticodePathInvalid,
    /// Windows did not produce an Authenticode signature.
    AuthenticodeFailed,
    /// A message input or caller-selected output bound could not fit a DWORD.
    MessageLimitInvalid,
    /// Windows could not create the requested attached CMS signature.
    MessageSigningFailed,
    /// The CMS envelope did not have exactly one usable signer.
    MessageVerificationFailed,
    /// Windows did not return a signer certificate after signature validation.
    MessageSignerUnavailable,
    /// Windows did not return one SHA-256 fingerprint for the message signer.
    MessageFingerprintUnavailable,
    /// The CMS signer was not the exact expected certificate.
    MessagePublisherMismatch,
}

impl fmt::Display for WindowsSigningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CertificateStoreUnavailable => "the current-user signing store is unavailable",
            Self::CertificateUnavailable => "the selected signing certificate is unavailable",
            Self::AuthenticodeUnavailable => "Windows Authenticode signing is unavailable",
            Self::AuthenticodePathInvalid => "the Authenticode signing path is invalid",
            Self::AuthenticodeFailed => "Windows could not sign the release image",
            Self::MessageLimitInvalid => "the signed message limit is invalid",
            Self::MessageSigningFailed => "Windows could not sign the update catalogue",
            Self::MessageVerificationFailed => "the signed update catalogue is invalid",
            Self::MessageSignerUnavailable => "the signed update catalogue has no signer",
            Self::MessageFingerprintUnavailable => {
                "the signed update catalogue signer is unavailable"
            }
            Self::MessagePublisherMismatch => {
                "the signed update catalogue used an unexpected publisher"
            }
        })
    }
}

impl std::error::Error for WindowsSigningError {}
