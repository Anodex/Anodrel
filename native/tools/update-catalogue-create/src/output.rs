//! Fresh synchronized JSON output for update catalogue authoring.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use crate::UpdateCatalogueCreateError;

/// Writes one new strict catalogue only after its input has been fully checked.
pub(super) fn write_new(output: &Path, contents: &[u8]) -> Result<(), UpdateCatalogueCreateError> {
    validate_output(output)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                UpdateCatalogueCreateError::OutputAlreadyExists
            } else {
                UpdateCatalogueCreateError::OutputCreationFailed
            }
        })?;
    let mut fresh = FreshOutput::new(output);
    let result = file
        .write_all(contents)
        .map_err(|_| UpdateCatalogueCreateError::OutputWriteFailed)
        .and_then(|()| {
            file.sync_all()
                .map_err(|_| UpdateCatalogueCreateError::OutputSyncFailed)
        });
    drop(file);
    result?;
    fresh.keep();
    Ok(())
}

fn validate_output(output: &Path) -> Result<(), UpdateCatalogueCreateError> {
    if !output.is_absolute() {
        return Err(UpdateCatalogueCreateError::OutputInvalid);
    }
    if output
        .try_exists()
        .map_err(|_| UpdateCatalogueCreateError::OutputInvalid)?
    {
        return Err(UpdateCatalogueCreateError::OutputAlreadyExists);
    }
    let parent = output
        .parent()
        .ok_or(UpdateCatalogueCreateError::OutputInvalid)?;
    let metadata =
        fs::symlink_metadata(parent).map_err(|_| UpdateCatalogueCreateError::OutputInvalid)?;
    if !metadata.is_dir() || is_link_like(&metadata) {
        return Err(UpdateCatalogueCreateError::OutputInvalid);
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
