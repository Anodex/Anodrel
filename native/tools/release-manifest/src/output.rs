//! Fresh synchronized output writing for release-manifest authoring.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use crate::ReleaseManifestAuthorError;

/// Writes only one new final manifest file and synchronizes it before success.
pub(super) fn write_new_manifest(
    output: &Path,
    manifest: &[u8],
) -> Result<(), ReleaseManifestAuthorError> {
    validate_output(output)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                ReleaseManifestAuthorError::OutputAlreadyExists
            } else {
                ReleaseManifestAuthorError::OutputCreationFailed
            }
        })?;
    let mut fresh = FreshOutput::new(output);
    let write_result = file
        .write_all(manifest)
        .map_err(|_| ReleaseManifestAuthorError::OutputWriteFailed)
        .and_then(|()| {
            file.sync_all()
                .map_err(|_| ReleaseManifestAuthorError::OutputSyncFailed)
        });
    drop(file);
    write_result?;
    fresh.keep();
    Ok(())
}

fn validate_output(output: &Path) -> Result<(), ReleaseManifestAuthorError> {
    if !output.is_absolute() {
        return Err(ReleaseManifestAuthorError::OutputInvalid);
    }
    if output
        .try_exists()
        .map_err(|_| ReleaseManifestAuthorError::OutputInvalid)?
    {
        return Err(ReleaseManifestAuthorError::OutputAlreadyExists);
    }
    let parent = output
        .parent()
        .ok_or(ReleaseManifestAuthorError::OutputInvalid)?;
    let metadata =
        fs::symlink_metadata(parent).map_err(|_| ReleaseManifestAuthorError::OutputInvalid)?;
    if !metadata.is_dir() || is_link_like(&metadata) {
        return Err(ReleaseManifestAuthorError::OutputInvalid);
    }
    Ok(())
}

/// Removes only the file created by this unsuccessful output operation.
struct FreshOutput<'path> {
    path: &'path Path,
    keep: bool,
}

impl<'path> FreshOutput<'path> {
    fn new(path: &'path Path) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for FreshOutput<'_> {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(self.path);
        }
    }
}

fn is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        metadata.file_attributes() & 0x0400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
