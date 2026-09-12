//! Recovery evidence and bounded, signed cache retirement under maintenance exclusion.

use super::{CleanupError, stage};
use crate::{ReleaseManifest, verify_locked_installer_image};
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

const COMMITTED: &str = "committed";

pub(super) fn mark_committed(directory: &Path) -> Result<PathBuf, CleanupError> {
    let path = directory.join(COMMITTED);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|file| file.sync_all())
        .map_err(|_| CleanupError::Cache)?;
    Ok(path)
}

/// Caller holds this application's maintenance lock throughout recovery.
pub(crate) fn retire(root: &Path, manifest: &ReleaseManifest) -> Result<(), CleanupError> {
    for entry in fs::read_dir(root).map_err(|_| CleanupError::Cache)? {
        let entry = entry.map_err(|_| CleanupError::Cache)?;
        if !stage::valid_name(&entry.file_name().to_string_lossy()) {
            continue;
        }
        retire_stage(root, &entry.path(), manifest)?;
    }
    Ok(())
}

fn retire_stage(
    root: &Path,
    directory: &Path,
    manifest: &ReleaseManifest,
) -> Result<(), CleanupError> {
    stage::normal(directory, true)?;
    // At most two entries, no recursion. Extra or unrecognized content fails closed.
    let entries = fs::read_dir(directory)
        .map_err(|_| CleanupError::Cache)?
        .take(3)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CleanupError::Cache)?;
    if entries.is_empty() {
        return fs::remove_dir(directory).map_err(|_| CleanupError::Cache);
    }
    if entries.len() > 2
        || entries
            .iter()
            .any(|e| e.file_name() != stage::IMAGE && e.file_name() != COMMITTED)
    {
        return Err(CleanupError::Cache);
    }
    let image = directory.join(stage::IMAGE);
    stage::normal(&image, false)?;
    let verified = verify_locked_installer_image(&image).map_err(|_| CleanupError::Verification)?;
    let cached = verified.manifest();
    if cached.application_id() != manifest.application_id()
        || cached.publisher_fingerprint() != manifest.publisher_fingerprint()
    {
        return Err(CleanupError::Verification);
    }
    let marker = directory.join(COMMITTED);
    let committed = entries.iter().any(|e| e.file_name() == COMMITTED);
    if committed {
        stage::normal(&marker, false)?;
        if fs::metadata(&marker)
            .map_err(|_| CleanupError::Cache)?
            .len()
            != 0
        {
            return Err(CleanupError::Cache);
        }
        match load_installed_application(manifest.application_id()) {
            Err(PolicyStoreError::RecordNotFound) => {
                let v = cached.package_version();
                let package = root.join(format!("{}.{}.{}", v.major(), v.minor(), v.patch()));
                match fs::symlink_metadata(&package) {
                    Ok(_) => crate::recovery::raw::remove_normal_tree(&package)
                        .map_err(|_| CleanupError::Package)?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => return Err(CleanupError::Package),
                }
            }
            // An interrupted transaction before policy removal grants no package deletion.
            Ok(_) => {}
            Err(_) => return Err(CleanupError::Policy),
        }
    }
    drop(verified);
    // Ordinary deletion refuses a mapped image. Failure must NOT authorize trust
    // removal. The caller must close the helper's result dialog and retry.
    if committed {
        fs::remove_file(marker).map_err(|_| CleanupError::Cache)?;
    }
    fs::remove_file(&image).map_err(|_| CleanupError::Cache)?;
    fs::remove_dir(directory).map_err(|_| CleanupError::Cache)
}
