//! Narrow discovery of private installer staging directories.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

#[cfg(windows)]
pub(crate) mod raw;

/// A recovery scan could not inspect one installer-owned application root.
#[derive(Debug)]
pub enum RecoveryDiscoveryError {
    /// The supplied installer-owned application root was not an absolute directory.
    ApplicationRootInvalid,
    /// Windows could not enumerate the installer-owned application root.
    DirectoryUnavailable,
}

impl fmt::Display for RecoveryDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ApplicationRootInvalid => "the installer application root is invalid",
            Self::DirectoryUnavailable => "the installer application root is unavailable",
        })
    }
}

impl std::error::Error for RecoveryDiscoveryError {}

/// Finds exact private stage directories without removing or modifying them.
pub fn discover_private_stages(
    application_root: &Path,
) -> Result<Vec<PathBuf>, RecoveryDiscoveryError> {
    let root = canonical_application_root(application_root)?;
    let entries = fs::read_dir(&root).map_err(|_| RecoveryDiscoveryError::DirectoryUnavailable)?;
    let mut stages = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| RecoveryDiscoveryError::DirectoryUnavailable)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !is_private_stage_name(name) {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| RecoveryDiscoveryError::DirectoryUnavailable)?;
        if !metadata.file_type().is_symlink() && metadata.is_dir() {
            stages.push(path);
        }
    }
    stages.sort();
    Ok(stages)
}

/// Removes every currently discovered private stage tree below one application root.
///
/// This removes no version directory or caller-named path. It uses direct Windows
/// enumeration and refuses reparse points before descending into a candidate.
#[cfg(windows)]
pub fn cleanup_private_stages(application_root: &Path) -> Result<usize, RecoveryCleanupError> {
    let stages =
        discover_private_stages(application_root).map_err(RecoveryCleanupError::DiscoveryFailed)?;
    for stage in &stages {
        raw::remove_normal_tree(stage)?;
    }
    Ok(stages.len())
}

/// Private-stage cleanup could not safely complete.
#[cfg(windows)]
#[derive(Debug)]
pub enum RecoveryCleanupError {
    /// Candidate discovery could not inspect the installer-owned root.
    DiscoveryFailed(RecoveryDiscoveryError),
    /// A candidate contained a link or junction that cleanup will not traverse.
    ReparsePointRefused,
    /// Windows could not enumerate or remove a checked private staging tree.
    RemovalFailed,
}

#[cfg(windows)]
impl fmt::Display for RecoveryCleanupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::DiscoveryFailed(_) => "private stage discovery could not complete",
            Self::ReparsePointRefused => "private stage cleanup refused a reparse point",
            Self::RemovalFailed => "private stage cleanup could not complete",
        };
        formatter.write_str(message)
    }
}

#[cfg(windows)]
impl std::error::Error for RecoveryCleanupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DiscoveryFailed(error) => Some(error),
            Self::ReparsePointRefused | Self::RemovalFailed => None,
        }
    }
}

fn canonical_application_root(path: &Path) -> Result<PathBuf, RecoveryDiscoveryError> {
    if !path.is_absolute() {
        return Err(RecoveryDiscoveryError::ApplicationRootInvalid);
    }
    let root =
        fs::canonicalize(path).map_err(|_| RecoveryDiscoveryError::ApplicationRootInvalid)?;
    fs::metadata(&root)
        .map_err(|_| RecoveryDiscoveryError::ApplicationRootInvalid)?
        .is_dir()
        .then_some(root)
        .ok_or(RecoveryDiscoveryError::ApplicationRootInvalid)
}

fn is_private_stage_name(name: &str) -> bool {
    let Some(suffix) = name.strip_prefix(".anodrel-stage-") else {
        return false;
    };
    let parts = suffix.split('-').collect::<Vec<_>>();
    parts.len() == 5 && parts.iter().all(|part| decimal_component(part))
}

fn decimal_component(value: &str) -> bool {
    !value.is_empty() && value.len() <= 20 && value.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::discover_private_stages;
    use crate::test_support::TestDirectory;

    #[cfg(windows)]
    use super::cleanup_private_stages;
    #[cfg(windows)]
    use super::raw;

    #[test]
    fn discovery_returns_only_exact_normal_private_stage_directories() {
        let root = TestDirectory::new("recovery");
        for name in [
            ".anodrel-stage-1-2-3-45-0",
            ".anodrel-stage-1-2-3-45-1",
            "1.2.3",
            ".anodrel-stage-1-2-three-45-2",
            ".anodrel-stage-1-2-3-45",
        ] {
            std::fs::create_dir(root.path().join(name)).expect("fixture directory is created");
        }
        std::fs::write(
            root.path().join(".anodrel-stage-1-2-3-45-3"),
            b"not a directory",
        )
        .expect("fixture file is created");

        let found = discover_private_stages(root.path()).expect("the root is scanned");
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|path| path.is_dir()));
        assert!(root.path().join("1.2.3").is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn cleanup_removes_only_discovered_private_stage_trees() {
        let root = TestDirectory::new("recovery");
        let stage = root.path().join(".anodrel-stage-1-2-3-45-0");
        std::fs::create_dir_all(stage.join("nested")).expect("the stage tree is created");
        std::fs::write(stage.join("nested/entry.txt"), b"stale data")
            .expect("the stage tree is populated");
        let version = root.path().join("1.2.3");
        std::fs::create_dir(&version).expect("the version directory is created");

        assert_eq!(
            cleanup_private_stages(root.path()).expect("cleanup succeeds"),
            1
        );
        assert!(!stage.exists());
        assert!(
            version.is_dir(),
            "version directories are never cleanup targets"
        );
    }

    #[cfg(windows)]
    #[test]
    fn uninstall_cleanup_retains_only_the_fixed_running_image() {
        let root = TestDirectory::new("recovery");
        let package = root.path().join("1.2.3");
        let installer = package
            .join("uninstaller")
            .join("anodrel-windows-installer.exe");
        std::fs::create_dir_all(installer.parent().expect("installer has parent"))
            .expect("installer directory is created");
        std::fs::write(&installer, b"running installer").expect("installer image is written");
        std::fs::create_dir_all(package.join("content")).expect("content directory is created");
        std::fs::write(
            package.join("content").join("main.txt"),
            b"application content",
        )
        .expect("content is written");
        std::fs::write(
            package.join("anodrel.application.json"),
            b"package manifest",
        )
        .expect("manifest is written");
        std::fs::write(package.join("uninstaller").join("stale.txt"), b"stale data")
            .expect("stale uninstaller data is written");

        raw::remove_normal_tree_except_installer(&package)
            .expect("only the running installer is retained");

        assert!(installer.is_file());
        assert!(package.join("uninstaller").is_dir());
        assert!(!package.join("content").exists());
        assert!(!package.join("anodrel.application.json").exists());
        assert!(!package.join("uninstaller").join("stale.txt").exists());
    }
}
