#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Fixed private Windows cache selection and recovery for update images.
//!
//! This adapter derives only `cache\\updates` from fixed machine policy and the
//! current user's Local AppData location. It creates and scans only normal
//! private directories and has no application protocol, general filesystem,
//! transfer, elevation, process, or installation surface. See
//! `docs/UPDATE_CACHE.md` and Decision 0170.

mod raw;

use std::{
    fmt,
    path::{Path, PathBuf},
};

use anodrel_paths::ApplicationDirectories;
use anodrel_windows_paths::{WindowsPathsError, application_directories};
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};

/// One fixed host-owned directory for private update images.
pub struct UpdateCache {
    directory: PathBuf,
}

impl UpdateCache {
    /// Returns the host-only absolute directory for fresh update images.
    ///
    /// This path must never become an application filesystem capability or a
    /// protocol, renderer, command-line, or environment value.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

impl fmt::Debug for UpdateCache {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UpdateCache(..)")
    }
}

/// The result of one constrained private-image recovery pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateCacheRecovery {
    removed_images: u32,
}

impl UpdateCacheRecovery {
    /// Returns the number of ordinary matching private images Windows removed.
    #[must_use]
    pub const fn removed_images(self) -> u32 {
        self.removed_images
    }
}

/// A fixed update cache could not be selected or scanned safely.
#[derive(Debug)]
pub enum UpdateCacheError {
    /// Fixed machine policy could not select one valid application identity.
    InstalledPolicyInvalid(PolicyStoreError),
    /// Windows could not derive the current user's host-owned application path.
    PathsInvalid(WindowsPathsError),
    /// The fixed cache hierarchy could not be created as normal directories.
    DirectoryInvalid,
    /// The fixed cache directory could not be enumerated safely.
    RecoveryUnavailable,
}

impl fmt::Display for UpdateCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InstalledPolicyInvalid(_) => "the installed application policy is invalid",
            Self::PathsInvalid(_) => "the private update cache location is unavailable",
            Self::DirectoryInvalid => "the private update cache directory is invalid",
            Self::RecoveryUnavailable => "the private update cache could not be recovered",
        })
    }
}

impl std::error::Error for UpdateCacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstalledPolicyInvalid(error) => Some(error),
            Self::PathsInvalid(error) => Some(error),
            Self::DirectoryInvalid | Self::RecoveryUnavailable => None,
        }
    }
}

/// Opens one fixed private cache for a host-selected installed application.
///
/// `application_id` must come from native updater composition, never an
/// application, protocol message, command line, environment value, or UI. This
/// function creates no image, runs no recovery, and does not start an update.
pub fn open_current_update_cache(application_id: &str) -> Result<UpdateCache, UpdateCacheError> {
    let installed = load_installed_application(application_id)
        .map_err(UpdateCacheError::InstalledPolicyInvalid)?;
    let directories =
        application_directories(installed.identity()).map_err(UpdateCacheError::PathsInvalid)?;
    open_cache_for_directories(&directories)
}

/// Removes only exact private image names from one already selected cache.
///
/// A locked, running, changed, directory, link, or unrelated file remains
/// untouched. A non-removable matching file is retained for another pass rather
/// than treated as a recovery failure or force-deleted.
pub fn recover_update_cache(cache: &UpdateCache) -> Result<UpdateCacheRecovery, UpdateCacheError> {
    let removed_images = raw::recover_private_images(cache.directory())
        .map_err(|_| UpdateCacheError::RecoveryUnavailable)?;
    Ok(UpdateCacheRecovery { removed_images })
}

fn open_cache_for_directories(
    directories: &ApplicationDirectories,
) -> Result<UpdateCache, UpdateCacheError> {
    let directory = directories.cache().join("updates");
    raw::ensure_normal_directory_tree(&directory)
        .map_err(|_| UpdateCacheError::DirectoryInvalid)?;
    Ok(UpdateCache { directory })
}

#[cfg(test)]
mod tests {
    use anodrel_application::ApplicationManifest;
    use anodrel_paths::ApplicationDirectories;
    use anodrel_windows_policy::PolicyStoreError;

    use super::{UpdateCacheError, open_cache_for_directories, open_current_update_cache};

    #[test]
    fn cache_is_a_fixed_child_of_the_existing_application_cache_namespace() {
        let root = std::env::temp_dir().join(format!(
            "anodrel-update-cache-layout-{}",
            std::process::id()
        ));
        let identity = identity();
        let directories = ApplicationDirectories::from_local_data_root(&root, &identity)
            .expect("fixture layout is valid");
        let cache = open_cache_for_directories(&directories).expect("cache is created");
        assert_eq!(cache.directory(), directories.cache().join("updates"));
        assert_eq!(format!("{cache:?}"), "UpdateCache(..)");
        std::fs::remove_dir_all(root).expect("fixture cache is removable");
    }

    #[test]
    fn invalid_identity_stops_before_path_or_directory_selection() {
        assert!(matches!(
            open_current_update_cache("org.anodrel/escape"),
            Err(UpdateCacheError::InstalledPolicyInvalid(
                PolicyStoreError::InvalidApplicationId
            ))
        ));
    }

    fn identity() -> anodrel_application::ApplicationIdentity {
        ApplicationManifest::parse(
            r#"{
                    "manifestVersion": { "major": 1, "minor": 0 },
                    "applicationId": "org.anodrel.update-cache-test",
                    "displayName": "Update Cache Test",
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
}
