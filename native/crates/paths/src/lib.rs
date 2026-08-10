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

/// Stable host-owned locations that belong to no application.
///
/// A sibling of the `Applications` namespace, for what the host records about
/// itself. A host defect is not an application's, and filing one under whichever
/// application happened to be loaded would both misattribute it and leak that
/// application's presence into another's directory. See Decision 0065.
///
/// This is a compatible extension to layout v1: it adds a namespace and reads
/// or creates nothing, exactly like [`ApplicationDirectories`].
#[derive(Clone, Eq, PartialEq)]
pub struct HostDirectories {
    host_root: PathBuf,
    logs: PathBuf,
}

impl HostDirectories {
    /// Derives the host's own locations below an absolute local-data root.
    ///
    /// # Errors
    ///
    /// Returns [`HostDirectoriesError::LocalDataRootNotAbsolute`] when the
    /// operating-system adapter supplied a relative root. There is no identity
    /// to validate: that is the point of this layout.
    pub fn from_local_data_root(local_data_root: &Path) -> Result<Self, HostDirectoriesError> {
        if !local_data_root.is_absolute() {
            return Err(HostDirectoriesError::LocalDataRootNotAbsolute);
        }
        let host_root = local_data_root.join("Anodrel").join("Host");
        Ok(Self {
            logs: host_root.join("logs"),
            host_root,
        })
    }

    /// Returns the stable root shared by the host's own locations.
    #[must_use]
    pub fn host_root(&self) -> &Path {
        &self.host_root
    }

    /// Returns the location for host-owned diagnostic records.
    ///
    /// A location, not a promise that a directory exists. Nothing here creates
    /// one; a writer owns its own creation and containment policy.
    #[must_use]
    pub fn logs(&self) -> &Path {
        &self.logs
    }
}

impl fmt::Debug for HostDirectories {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HostDirectories(..)")
    }
}

/// A safe category for a host-directory layout failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostDirectoriesError {
    /// The operating-system adapter returned a relative root.
    LocalDataRootNotAbsolute,
}

impl fmt::Display for HostDirectoriesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("host local-data root is not absolute")
    }
}

impl std::error::Error for HostDirectoriesError {}

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

    use super::{
        ApplicationDirectories, ApplicationDirectoriesError, HostDirectories, HostDirectoriesError,
    };

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

        let host = HostDirectories::from_local_data_root(Path::new(
            r"C:\Anodrel-paths-test-root-that-does-not-exist",
        ))
        .expect("fixture host directories are valid");
        assert_eq!(format!("{host:?}"), "HostDirectories(..)");
    }

    #[test]
    fn derives_the_host_locations_without_touching_the_filesystem() {
        let root = Path::new(r"C:\Anodrel-paths-test-root-that-does-not-exist");
        assert!(!root.exists(), "fixture root must not already exist");

        let host =
            HostDirectories::from_local_data_root(root).expect("an absolute root is accepted");
        assert_eq!(
            host.host_root(),
            Path::new(r"C:\Anodrel-paths-test-root-that-does-not-exist\Anodrel\Host")
        );
        assert_eq!(
            host.logs(),
            Path::new(r"C:\Anodrel-paths-test-root-that-does-not-exist\Anodrel\Host\logs")
        );
        assert!(!host.host_root().exists());
    }

    #[test]
    fn the_host_namespace_cannot_collide_with_an_application() {
        // `Host` sits beside `Applications`, not inside it, so no application
        // identity can ever resolve to the host's own location. The identity
        // grammar forbids a path separator, but this is the property that
        // matters and it should be asserted rather than inferred.
        let root = Path::new(r"C:\Anodrel-paths-test-root-that-does-not-exist");
        let host = HostDirectories::from_local_data_root(root).expect("host layout is valid");
        let application = ApplicationDirectories::from_local_data_root(root, &identity())
            .expect("application layout is valid");
        assert!(
            !host
                .host_root()
                .starts_with(root.join("Anodrel\\Applications"))
        );
        assert!(!application.application_root().starts_with(host.host_root()));
        assert_ne!(host.logs(), application.logs());
    }

    #[test]
    fn the_host_layout_rejects_a_relative_operating_system_root() {
        assert_eq!(
            HostDirectories::from_local_data_root(Path::new("local-data")),
            Err(HostDirectoriesError::LocalDataRootNotAbsolute)
        );
    }
}
