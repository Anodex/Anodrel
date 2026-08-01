#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows lookup for host-owned application directories.
//!
//! The adapter reads the current user's Local AppData known folder and derives
//! one application's stable locations. It performs no directory I/O and does
//! not expose filesystem authority to a protocol client. See `docs/PATHS.md`.

mod raw;

use std::fmt;

use anodrel_application::ApplicationIdentity;
use anodrel_paths::{ApplicationDirectories, ApplicationDirectoriesError};

/// Derives stable host-owned locations for one validated application identity.
pub fn application_directories(
    identity: &ApplicationIdentity,
) -> Result<ApplicationDirectories, WindowsPathsError> {
    let root = raw::local_application_data_root().map_err(WindowsPathsError::from)?;
    ApplicationDirectories::from_local_data_root(&root, identity).map_err(WindowsPathsError::Layout)
}

/// A safe category for a Windows application-directory lookup failure.
#[derive(Debug)]
pub enum WindowsPathsError {
    LocalDataUnavailable,
    LocalDataPathInvalid,
    Layout(ApplicationDirectoriesError),
}

impl From<raw::KnownFolderError> for WindowsPathsError {
    fn from(error: raw::KnownFolderError) -> Self {
        match error {
            raw::KnownFolderError::Unavailable => Self::LocalDataUnavailable,
            raw::KnownFolderError::InvalidPath => Self::LocalDataPathInvalid,
        }
    }
}

impl fmt::Display for WindowsPathsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::LocalDataUnavailable => "Windows local application data is unavailable",
            Self::LocalDataPathInvalid => "Windows local application data path is invalid",
            Self::Layout(_) => "application directory layout could not be derived",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WindowsPathsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Layout(error) => Some(error),
            Self::LocalDataUnavailable | Self::LocalDataPathInvalid => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_application::ApplicationManifest;

    use super::application_directories;

    #[test]
    fn reads_the_current_user_root_without_creating_a_directory() {
        let manifest = ApplicationManifest::parse(
            r#"{
                "manifestVersion": { "major": 1, "minor": 0 },
                "applicationId": "org.anodrel.windows-paths-test",
                "displayName": "Windows Paths Test",
                "content": {
                    "format": "anodrel.text.v1",
                    "path": "content/main.txt",
                    "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                }
            }"#,
        )
        .expect("fixture manifest is valid");
        let directories = application_directories(manifest.identity())
            .expect("Windows provides Local AppData for the current user");

        assert!(directories.application_root().is_absolute());
        assert_eq!(
            directories
                .data()
                .file_name()
                .and_then(|name| name.to_str()),
            Some("data")
        );
        assert_eq!(
            directories
                .cache()
                .file_name()
                .and_then(|name| name.to_str()),
            Some("cache")
        );
        assert_eq!(
            directories
                .logs()
                .file_name()
                .and_then(|name| name.to_str()),
            Some("logs")
        );
        assert!(!directories.application_root().exists());
    }
}
