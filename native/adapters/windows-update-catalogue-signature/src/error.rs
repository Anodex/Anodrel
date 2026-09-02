//! Closed failure categories for signed update catalogues.

use std::fmt;

use anodrel_update_catalogue::UpdateCatalogueError;
use anodrel_windows_signing::WindowsSigningError;

/// An update catalogue could not become one checked signed authority input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateCatalogueSignatureError {
    /// The decoded catalogue did not meet the strict portable contract.
    CatalogueInvalid(UpdateCatalogueError),
    /// Windows could not create or verify the attached CMS message.
    SignatureInvalid(WindowsSigningError),
}

impl fmt::Display for UpdateCatalogueSignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CatalogueInvalid(_) => "the update catalogue is invalid",
            Self::SignatureInvalid(_) => "the signed update catalogue is invalid",
        })
    }
}

impl std::error::Error for UpdateCatalogueSignatureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CatalogueInvalid(error) => Some(error),
            Self::SignatureInvalid(error) => Some(error),
        }
    }
}
