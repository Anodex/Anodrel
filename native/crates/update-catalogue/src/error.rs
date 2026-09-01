//! Closed parsing failure categories for update catalogues.

use std::fmt;

/// A future update catalogue did not meet the exact version-1 contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateCatalogueError {
    /// The UTF-8 catalogue exceeded its fixed bounded size.
    TooLarge,
    /// The catalogue was not strict UTF-8 JSON with the required field shape.
    Invalid,
    /// The catalogue named an unsupported format version.
    VersionUnsupported,
    /// The catalogue's one installer location was not an exact HTTPS path.
    InstallerLocationInvalid,
    /// The catalogue's installer byte descriptor was out of its fixed bounds.
    InstallerBytesInvalid,
}

impl fmt::Display for UpdateCatalogueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TooLarge => "the update catalogue exceeds its fixed size limit",
            Self::Invalid => "the update catalogue is invalid",
            Self::VersionUnsupported => "the update catalogue version is unsupported",
            Self::InstallerLocationInvalid => "the update installer location is invalid",
            Self::InstallerBytesInvalid => "the update installer bytes are invalid",
        })
    }
}

impl std::error::Error for UpdateCatalogueError {}
