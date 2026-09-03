//! Closed failure categories for release-manifest authoring.

use std::fmt;

use anodrel_application::ApplicationError;
use anodrel_release_bundle::ReleaseBundleError;
use anodrel_windows_installer::{ReleaseManifestError, ReleasePayloadError};

/// A final release manifest could not be authored safely.
#[derive(Debug)]
pub enum ReleaseManifestAuthorError {
    /// A plan or bundle input path was not an absolute normal regular file.
    InputInvalid,
    /// A plan or bundle input file could not be read within its fixed limit.
    InputReadFailed,
    /// The release plan was not strict version-1 authoring input.
    PlanInvalid,
    /// The owned release bundle did not meet its checked byte contract.
    BundleInvalid(ReleaseBundleError),
    /// The bundle did not contain the required root application manifest.
    ApplicationManifestUnavailable,
    /// The bundled application manifest did not meet its strict contract.
    ApplicationManifestInvalid(ApplicationError),
    /// The bundled application content was absent or did not meet its contract.
    ApplicationContentInvalid,
    /// The release-plan executable did not name one bundle entry.
    ExecutableUnavailable,
    /// The planned product launcher did not name one checked bundle entry.
    LauncherUnavailable,
    /// The derived final manifest did not meet the owned release contract.
    ManifestInvalid(ReleaseManifestError),
    /// The checked final manifest did not match the source bundle bytes.
    PayloadInvalid(ReleasePayloadError),
    /// The requested output was not an absolute fresh path with a normal parent.
    OutputInvalid,
    /// The requested output already existed and must remain unchanged.
    OutputAlreadyExists,
    /// The fresh output file could not be created.
    OutputCreationFailed,
    /// The fresh output file could not receive all manifest bytes.
    OutputWriteFailed,
    /// The fresh output file could not be synchronized.
    OutputSyncFailed,
}

impl fmt::Display for ReleaseManifestAuthorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputInvalid => "the release manifest input path is invalid",
            Self::InputReadFailed => "the release manifest input could not be read",
            Self::PlanInvalid => "the release plan is invalid",
            Self::BundleInvalid(_) => "the release bundle is invalid",
            Self::ApplicationManifestUnavailable => {
                "the release bundle has no application manifest"
            }
            Self::ApplicationManifestInvalid(_) => "the bundled application manifest is invalid",
            Self::ApplicationContentInvalid => "the bundled application content is invalid",
            Self::ExecutableUnavailable => {
                "the planned executable is unavailable in the release bundle"
            }
            Self::LauncherUnavailable => {
                "the planned product launcher is unavailable in the release bundle"
            }
            Self::ManifestInvalid(_) => "the derived release manifest is invalid",
            Self::PayloadInvalid(_) => "the derived release payload is invalid",
            Self::OutputInvalid => "the release manifest output path is invalid",
            Self::OutputAlreadyExists => "the release manifest output already exists",
            Self::OutputCreationFailed => "the release manifest output could not be created",
            Self::OutputWriteFailed => "the release manifest output could not be written",
            Self::OutputSyncFailed => "the release manifest output could not be synchronized",
        })
    }
}

impl std::error::Error for ReleaseManifestAuthorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BundleInvalid(error) => Some(error),
            Self::ApplicationManifestInvalid(error) => Some(error),
            Self::ManifestInvalid(error) => Some(error),
            Self::PayloadInvalid(error) => Some(error),
            Self::InputInvalid
            | Self::InputReadFailed
            | Self::PlanInvalid
            | Self::ApplicationManifestUnavailable
            | Self::ApplicationContentInvalid
            | Self::ExecutableUnavailable
            | Self::LauncherUnavailable
            | Self::OutputInvalid
            | Self::OutputAlreadyExists
            | Self::OutputCreationFailed
            | Self::OutputWriteFailed
            | Self::OutputSyncFailed => None,
        }
    }
}
