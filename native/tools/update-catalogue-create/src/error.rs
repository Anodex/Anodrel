//! Closed safe failures from locked-image update catalogue authoring.

use std::fmt;

use anodrel_windows_installer::InstallerImageError;
use anodrel_windows_signature::SignatureError;

/// One failure while creating an unsigned strict update catalogue.
#[derive(Debug)]
pub enum UpdateCatalogueCreateError {
    /// The installer input was not an absolute normal regular image.
    InputInvalid,
    /// The installer input could not be measured within the image limit.
    InputReadFailed,
    /// The installer image did not pass locked release and signature acceptance.
    ImageInvalid(InstallerImageError),
    /// Windows did not return an accepted signer for the locked image.
    SignerInvalid(SignatureError),
    /// The explicit publication location did not meet strict catalogue grammar.
    LocationInvalid,
    /// The derived catalogue did not re-parse through its own strict contract.
    CatalogueInvalid,
    /// The requested output was not an absolute fresh path below a normal parent.
    OutputInvalid,
    /// The requested output already exists and remains untouched.
    OutputAlreadyExists,
    /// The fresh output file could not be created.
    OutputCreationFailed,
    /// The fresh output file could not receive every byte.
    OutputWriteFailed,
    /// The fresh output file could not be synchronized.
    OutputSyncFailed,
}

impl fmt::Display for UpdateCatalogueCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputInvalid => "the signed installer input path is invalid",
            Self::InputReadFailed => "the signed installer input could not be measured",
            Self::ImageInvalid(_) => "the signed installer image could not be accepted",
            Self::SignerInvalid(_) => "Windows did not accept the signed installer publisher",
            Self::LocationInvalid => "the update publication location is invalid",
            Self::CatalogueInvalid => "the derived update catalogue is invalid",
            Self::OutputInvalid => "the update catalogue output path is invalid",
            Self::OutputAlreadyExists => "the update catalogue output already exists",
            Self::OutputCreationFailed => "the update catalogue output could not be created",
            Self::OutputWriteFailed => "the update catalogue output could not be written",
            Self::OutputSyncFailed => "the update catalogue output could not be synchronized",
        })
    }
}

impl std::error::Error for UpdateCatalogueCreateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ImageInvalid(error) => Some(error),
            Self::SignerInvalid(error) => Some(error),
            Self::InputInvalid
            | Self::InputReadFailed
            | Self::LocationInvalid
            | Self::CatalogueInvalid
            | Self::OutputInvalid
            | Self::OutputAlreadyExists
            | Self::OutputCreationFailed
            | Self::OutputWriteFailed
            | Self::OutputSyncFailed => None,
        }
    }
}
