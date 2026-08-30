//! Linux current-user root derivation and portable Anodrel layout composition.

mod raw;

use std::{fmt, path::PathBuf};

use anodrel_application::ApplicationIdentity;
use anodrel_paths::{ApplicationDirectories, HostDirectories};

/// Derives one Linux application layout for a validated application identity.
pub fn application_directories(
    identity: &ApplicationIdentity,
) -> Result<ApplicationDirectories, LinuxPathsError> {
    let root = local_data_root()?;
    ApplicationDirectories::from_local_data_root(&root, identity)
        .map_err(|_| LinuxPathsError::LocalDataPathInvalid)
}

/// Derives Linux host locations that belong to no application.
pub fn host_directories() -> Result<HostDirectories, LinuxPathsError> {
    let root = local_data_root()?;
    HostDirectories::from_local_data_root(&root).map_err(|_| LinuxPathsError::LocalDataPathInvalid)
}

/// Returns the host-only default local-data root for the effective account.
///
/// This creates no directory and reads neither environment variables nor the
/// current working directory.
pub fn local_data_root() -> Result<PathBuf, LinuxPathsError> {
    let home = raw::effective_home_directory().map_err(LinuxPathsError::from)?;
    local_data_root_from_home(home)
}

fn local_data_root_from_home(home: PathBuf) -> Result<PathBuf, LinuxPathsError> {
    if !home.is_absolute() {
        return Err(LinuxPathsError::LocalDataPathInvalid);
    }
    Ok(home.join(".local").join("share"))
}

/// Closed safe categories for Linux current-user path lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxPathsError {
    /// The effective account record could not be read.
    AccountUnavailable,
    /// The effective account home or derived local-data root was invalid.
    LocalDataPathInvalid,
}

impl From<raw::AccountLookupError> for LinuxPathsError {
    fn from(error: raw::AccountLookupError) -> Self {
        match error {
            raw::AccountLookupError::Unavailable => Self::AccountUnavailable,
            raw::AccountLookupError::InvalidHome => Self::LocalDataPathInvalid,
        }
    }
}

impl fmt::Display for LinuxPathsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccountUnavailable => formatter.write_str("current Linux account is unavailable"),
            Self::LocalDataPathInvalid => formatter.write_str("Linux local-data root is invalid"),
        }
    }
}

impl std::error::Error for LinuxPathsError {}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anodrel_application::ApplicationManifest;

    use super::{
        LinuxPathsError, application_directories, host_directories, local_data_root_from_home,
    };

    fn identity() -> anodrel_application::ApplicationIdentity {
        ApplicationManifest::parse(
            r#"{
                "manifestVersion": { "major": 1, "minor": 0 },
                "applicationId": "org.anodrel.linux-paths-test",
                "displayName": "Linux Paths Test",
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
    fn fixed_default_root_requires_an_absolute_effective_home() {
        assert_eq!(
            local_data_root_from_home(PathBuf::from("relative-home")),
            Err(LinuxPathsError::LocalDataPathInvalid)
        );
        assert_eq!(
            local_data_root_from_home(PathBuf::from("/home/anodrel")),
            Ok(PathBuf::from("/home/anodrel/.local/share"))
        );
    }

    #[test]
    fn current_account_layouts_are_host_owned_and_separate() {
        let application = application_directories(&identity())
            .expect("current effective account has a valid home");
        let host = host_directories().expect("current effective account has a valid home");
        assert!(application.data().is_absolute());
        assert!(application.cache().is_absolute());
        assert!(application.logs().is_absolute());
        assert!(host.logs().is_absolute());
        assert!(!application.application_root().starts_with(host.host_root()));
    }
}
