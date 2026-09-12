//! Protected helper images, independently verified before either side commits.

use super::CleanupError;
use crate::{ReleaseManifest, VerifiedUninstallTarget, verify_locked_installer_image};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const PREFIX: &str = ".anodrel-cleanup-";
pub(super) const IMAGE: &str = "anodrel-windows-installer.exe";
const MAX_STAGES: usize = 16;

pub(super) struct Stage {
    directory: PathBuf,
    image: PathBuf,
    _lock: crate::VerifiedInstallerImage,
}
impl Stage {
    pub(super) fn image(&self) -> &Path {
        &self.image
    }
    pub(super) fn directory(&self) -> &Path {
        &self.directory
    }
}

pub(super) fn prepare(
    target: &VerifiedUninstallTarget,
    manifest: &ReleaseManifest,
) -> Result<Stage, CleanupError> {
    let root = target.package_root().parent().ok_or(CleanupError::Cache)?;
    super::cache::retire(root, manifest)?;
    if fs::read_dir(root)
        .map_err(|_| CleanupError::Cache)?
        .filter_map(Result::ok)
        .filter(|e| valid_name(&e.file_name().to_string_lossy()))
        .count()
        >= MAX_STAGES
    {
        return Err(CleanupError::Cache);
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CleanupError::Cache)?
        .as_nanos();
    let directory = root.join(format!("{PREFIX}{}-{stamp}", std::process::id()));
    // Exclusive creation; never reuse an earlier transaction's directory.
    fs::create_dir(&directory).map_err(|_| CleanupError::Cache)?;
    normal(&directory, true)?;
    let image = directory.join(IMAGE);
    let source = crate::installed_uninstaller::installed_uninstaller_path(target.package_root());
    let result = (|| {
        let _source_lock =
            verify_locked_installer_image(&source).map_err(|_| CleanupError::Verification)?;
        crate::installed_uninstaller::copy_current_image(&source, &image)
            .map_err(|_| CleanupError::Cache)?;
        let lock = verify_locked_installer_image(&image).map_err(|_| CleanupError::Verification)?;
        if lock.manifest().application_id() != manifest.application_id()
            || lock.manifest().package_version() != manifest.package_version()
            || lock.manifest().publisher_fingerprint() != manifest.publisher_fingerprint()
        {
            return Err(CleanupError::Verification);
        }
        Ok(Stage {
            directory: directory.clone(),
            image: image.clone(),
            _lock: lock,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&image);
        let _ = fs::remove_dir(&directory);
    }
    result
}

pub(super) fn verify_helper() -> Result<VerifiedUninstallTarget, CleanupError> {
    let target = crate::uninstall::verify_selected_uninstall_target()
        .map_err(|_| CleanupError::Verification)?;
    let current = std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|_| CleanupError::Verification)?;
    let dir = current.parent().ok_or(CleanupError::Verification)?;
    if current.file_name() != Some(std::ffi::OsStr::new(IMAGE))
        || dir.parent() != target.package_root().parent()
        || !dir
            .file_name()
            .is_some_and(|n| valid_name(&n.to_string_lossy()))
    {
        return Err(CleanupError::Verification);
    }
    normal(dir, true)?;
    normal(target.package_root(), true)?;
    normal(&current, false)?;
    let installed = crate::installed_uninstaller::installed_uninstaller_path(target.package_root());
    normal(installed.parent().ok_or(CleanupError::Verification)?, true)?;
    normal(&installed, false)?;
    let _lock =
        verify_locked_installer_image(&installed).map_err(|_| CleanupError::Verification)?;
    if !same_bytes(&current, &installed).map_err(|_| CleanupError::Verification)? {
        return Err(CleanupError::Verification);
    }
    Ok(target)
}

pub(super) fn normal(path: &Path, directory: bool) -> Result<(), CleanupError> {
    let info = fs::symlink_metadata(path).map_err(|_| CleanupError::Verification)?;
    (info.file_attributes() & 0x400 == 0
        && info.is_dir() == directory
        && (directory || info.is_file()))
    .then_some(())
    .ok_or(CleanupError::Verification)
}
fn same_bytes(left: &Path, right: &Path) -> io::Result<bool> {
    let mut a = OpenOptions::new().read(true).share_mode(1).open(left)?;
    let mut b = File::open(right)?;
    if a.metadata()?.len() != b.metadata()?.len() {
        return Ok(false);
    }
    let mut x = [0; 16384];
    let mut y = [0; 16384];
    loop {
        let count = a.read(&mut x)?;
        if count == 0 {
            return Ok(true);
        }
        b.read_exact(&mut y[..count])?;
        if x[..count] != y[..count] {
            return Ok(false);
        }
    }
}
pub(super) fn valid_name(name: &str) -> bool {
    name.strip_prefix(PREFIX).is_some_and(|tail| {
        let mut fields = tail.split('-');
        let decimal =
            |s: &str| !s.is_empty() && s.len() <= 39 && s.bytes().all(|b| b.is_ascii_digit());
        fields.next().is_some_and(decimal)
            && fields.next().is_some_and(decimal)
            && fields.next().is_none()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_selection_excludes_packages_and_other_staging_trees() {
        assert!(valid_name(".anodrel-cleanup-123-456789"));
        for n in [
            "0.1.0",
            ".anodrel-stage-1-2-3-4-5",
            ".anodrel-cleanup-",
            ".anodrel-cleanup-1-../other",
            ".anodrel-cleanup-1-2-3",
            ".anodrel-cleanup-1-",
        ] {
            assert!(!valid_name(n));
        }
    }
    #[test]
    fn helper_copy_comparison_checks_every_byte_and_length() {
        let dir = crate::test_support::TestDirectory::new("cleanup-copy");
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::write(&a, vec![42; 40000]).unwrap();
        fs::write(&b, vec![42; 40000]).unwrap();
        assert!(same_bytes(&a, &b).unwrap());
        fs::write(&b, vec![43; 40000]).unwrap();
        assert!(!same_bytes(&a, &b).unwrap());
        fs::write(&b, b"short").unwrap();
        assert!(!same_bytes(&a, &b).unwrap());
    }
}
