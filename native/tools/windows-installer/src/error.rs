//! Safe release-manifest failure categories.

use std::fmt;

/// A release manifest failed its strict format or policy checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseManifestError {
    /// The input exceeded the fixed manifest size limit.
    TooLarge,
    /// The input was not strict JSON in the documented manifest shape.
    Invalid,
    /// The manifest named an unsupported release-envelope version.
    VersionUnsupported,
    /// The manifest named a malformed or unsafe executable path.
    ExecutablePathInvalid,
    /// The manifest's capabilities and network origins did not form policy.
    PolicyInvalid,
    /// The manifest's payload descriptor exceeded its fixed bounds.
    PayloadInvalid,
}

impl fmt::Display for ReleaseManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooLarge => "release manifest exceeds its limit",
            Self::Invalid => "release manifest is invalid",
            Self::VersionUnsupported => "release manifest version is unsupported",
            Self::ExecutablePathInvalid => "release manifest executable path is invalid",
            Self::PolicyInvalid => "release manifest policy is invalid",
            Self::PayloadInvalid => "release manifest payload is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ReleaseManifestError {}
