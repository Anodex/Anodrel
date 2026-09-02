//! Fresh synchronized signed-catalogue output writing.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use crate::UpdateCatalogueSignToolError;

/// Writes one new signed catalogue file and synchronizes it before success.
pub(super) fn write_new(output: &Path, bytes: &[u8]) -> Result<(), UpdateCatalogueSignToolError> {
    validate_output(output)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                UpdateCatalogueSignToolError::OutputAlreadyExists
            } else {
                UpdateCatalogueSignToolError::OutputCreationFailed
            }
        })?;
    let mut fresh = FreshOutput::new(output);
    let write_result = file
        .write_all(bytes)
        .map_err(|_| UpdateCatalogueSignToolError::OutputWriteFailed)
        .and_then(|()| {
            file.sync_all()
                .map_err(|_| UpdateCatalogueSignToolError::OutputSyncFailed)
        });
    drop(file);
    write_result?;
    fresh.keep();
    Ok(())
}

/// Validates an output before the signer opens a selected certificate.
pub(super) fn validate_output(output: &Path) -> Result<(), UpdateCatalogueSignToolError> {
    if !output.is_absolute() {
        return Err(UpdateCatalogueSignToolError::OutputInvalid);
    }
    if output
        .try_exists()
        .map_err(|_| UpdateCatalogueSignToolError::OutputInvalid)?
    {
        return Err(UpdateCatalogueSignToolError::OutputAlreadyExists);
    }
    let parent = output
        .parent()
        .ok_or(UpdateCatalogueSignToolError::OutputInvalid)?;
    let metadata =
        fs::symlink_metadata(parent).map_err(|_| UpdateCatalogueSignToolError::OutputInvalid)?;
    if !metadata.is_dir() || is_link_like(&metadata) {
        return Err(UpdateCatalogueSignToolError::OutputInvalid);
    }
    Ok(())
}

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
