//! Closed authoring failures for signed update catalogues.

use std::fmt;

use anodrel_windows_update_catalogue_signature::UpdateCatalogueSignatureError;

/// A signed update catalogue could not be authored safely.
#[derive(Debug)]
pub enum UpdateCatalogueSignToolError {
    /// The input path was not one absolute normal regular file.
    InputInvalid,
    /// The input could not be read within its fixed bound.
    InputReadFailed,
    /// The certificate fingerprint was not lowercase SHA-256 text.
    CertificateFingerprintInvalid,
    /// The strict catalogue or its attached CMS signature was invalid.
    SignatureInvalid(UpdateCatalogueSignatureError),
    /// The output was not absolute, fresh, or below a normal parent directory.
    OutputInvalid,
    /// The requested output already existed and must remain unchanged.
    OutputAlreadyExists,
    /// The fresh output file could not be created.
    OutputCreationFailed,
    /// The fresh output file could not receive all signed bytes.
    OutputWriteFailed,
    /// The fresh output file could not be synchronized.
    OutputSyncFailed,
}

impl fmt::Display for UpdateCatalogueSignToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputInvalid => "the update catalogue input path is invalid",
            Self::InputReadFailed => "the update catalogue input could not be read",
            Self::CertificateFingerprintInvalid => "the signing certificate fingerprint is invalid",
            Self::SignatureInvalid(_) => "the update catalogue could not be signed",
            Self::OutputInvalid => "the signed update catalogue output path is invalid",
            Self::OutputAlreadyExists => "the signed update catalogue output already exists",
            Self::OutputCreationFailed => "the signed update catalogue output could not be created",
            Self::OutputWriteFailed => "the signed update catalogue output could not be written",
            Self::OutputSyncFailed => {
                "the signed update catalogue output could not be synchronized"
            }
        })
    }
}

impl std::error::Error for UpdateCatalogueSignToolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SignatureInvalid(error) => Some(error),
            Self::InputInvalid
            | Self::InputReadFailed
            | Self::CertificateFingerprintInvalid
            | Self::OutputInvalid
            | Self::OutputAlreadyExists
            | Self::OutputCreationFailed
            | Self::OutputWriteFailed
            | Self::OutputSyncFailed => None,
        }
    }
}
