//! Narrow discovery of private installer staging directories.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

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
    use std::path::PathBuf;

    use super::discover_private_stages;

    #[test]
    fn discovery_returns_only_exact_normal_private_stage_directories() {
        let root = TemporaryDirectory::new();
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

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "anodrel-recovery-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("the system time is after the epoch")
                    .as_nanos()
            ));
            std::fs::create_dir(&path).expect("the test root is created");
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
