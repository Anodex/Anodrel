#![forbid(unsafe_code)]

//! Host-owned application directory layout.
//!
//! This crate derives stable directory locations from an operating-system root
//! and a validated application identity. It does not touch the filesystem,
//! create directories, or expose a public application protocol. See
//! `docs/PATHS.md`.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use anodrel_application::{ApplicationIdentity, is_valid_application_id};

/// Stable host-owned locations for one validated application identity.
#[derive(Clone, Eq, PartialEq)]
pub struct ApplicationDirectories {
    application_root: PathBuf,
    data: PathBuf,
    cache: PathBuf,
    logs: PathBuf,
}

impl ApplicationDirectories {
    /// Derives v1 locations below an absolute local-data root without I/O.
    pub fn from_local_data_root(
        local_data_root: &Path,
        identity: &ApplicationIdentity,
    ) -> Result<Self, ApplicationDirectoriesError> {
        if !local_data_root.is_absolute() {
            return Err(ApplicationDirectoriesError::LocalDataRootNotAbsolute);
        }
        if !is_valid_application_id(identity.application_id()) {
            return Err(ApplicationDirectoriesError::InvalidApplicationIdentity);
        }

        let application_root = local_data_root
            .join("Anodrel")
            .join("Applications")
            .join(identity.application_id());
        Ok(Self {
            data: application_root.join("data"),
            cache: application_root.join("cache"),
            logs: application_root.join("logs"),
            application_root,
        })
    }

    /// Returns the stable root shared by this application's v1 locations.
    #[must_use]
    pub fn application_root(&self) -> &Path {
        &self.application_root
    }

    /// Returns the location for durable application data.
    #[must_use]
    pub fn data(&self) -> &Path {
        &self.data
    }

    /// Returns the location for disposable cache data.
    #[must_use]
    pub fn cache(&self) -> &Path {
        &self.cache
    }

    /// Returns the location reserved for host-managed application logs.
    #[must_use]
    pub fn logs(&self) -> &Path {
        &self.logs
    }
}

impl fmt::Debug for ApplicationDirectories {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ApplicationDirectories(..)")
    }
}

/// A safe category for an application-directory layout failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationDirectoriesError {
    /// The operating-system adapter returned a relative root.
    LocalDataRootNotAbsolute,
    /// The identity no longer satisfies Anodrel's shared grammar.
    InvalidApplicationIdentity,
}

impl fmt::Display for ApplicationDirectoriesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::LocalDataRootNotAbsolute => "application local-data root is not absolute",
            Self::InvalidApplicationIdentity => {
                "application identity is invalid for directory layout"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ApplicationDirectoriesError {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anodrel_application::ApplicationManifest;

    use super::{ApplicationDirectories, ApplicationDirectoriesError};

    fn identity() -> anodrel_application::ApplicationIdentity {
        ApplicationManifest::parse(
            r#"{
                "manifestVersion": { "major": 1, "minor": 0 },
                "applicationId": "org.anodrel.paths-test",
                "displayName": "Paths Test",
                "content": {
                    "format": "anodrel.text.v1",
                    "path": "content/main.txt",
                    "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                }
            }"#,
        )
        .expect("fixture manifest is valid")
        .identity()
        .clone()
    }

    #[test]
    fn derives_the_fixed_v1_locations_without_touching_the_filesystem() {
        let root = Path::new(r"C:\Anodrel-paths-test-root-that-does-not-exist");
        assert!(!root.exists(), "fixture root must not already exist");

        let directories = ApplicationDirectories::from_local_data_root(root, &identity())
            .expect("absolute root and validated identity are accepted");
        assert_eq!(
            directories.application_root(),
            Path::new(
                r"C:\Anodrel-paths-test-root-that-does-not-exist\Anodrel\Applications\org.anodrel.paths-test"
            )
        );
        assert_eq!(
            directories.data(),
            Path::new(
                r"C:\Anodrel-paths-test-root-that-does-not-exist\Anodrel\Applications\org.anodrel.paths-test\data"
            )
        );
        assert_eq!(
            directories.cache(),
            Path::new(
                r"C:\Anodrel-paths-test-root-that-does-not-exist\Anodrel\Applications\org.anodrel.paths-test\cache"
            )
        );
        assert_eq!(
            directories.logs(),
            Path::new(
                r"C:\Anodrel-paths-test-root-that-does-not-exist\Anodrel\Applications\org.anodrel.paths-test\logs"
            )
        );
        assert!(!directories.application_root().exists());
    }

    #[test]
    fn rejects_a_relative_operating_system_root() {
        assert_eq!(
            ApplicationDirectories::from_local_data_root(Path::new("local-data"), &identity()),
            Err(ApplicationDirectoriesError::LocalDataRootNotAbsolute)
        );
    }

    #[test]
    fn debug_output_does_not_reveal_an_absolute_path() {
        let directories = ApplicationDirectories::from_local_data_root(
            Path::new(r"C:\Anodrel-paths-test-root-that-does-not-exist"),
            &identity(),
        )
        .expect("fixture directories are valid");
        assert_eq!(format!("{directories:?}"), "ApplicationDirectories(..)");
    }
}
