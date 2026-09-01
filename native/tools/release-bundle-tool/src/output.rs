//! Fresh-output creation and synchronization for bundle authoring.

use std::{fs::OpenOptions, io::Write, path::Path};

use anodrel_release_bundle::{BundleEntryInput, ReleaseBundle, encode};

use crate::{BundleAuthorError, source::SourceEntry};

/// Encodes, validates, and writes one fresh synchronized output file.
pub(super) fn write_new_bundle(
    output: &Path,
    source: &[SourceEntry],
) -> Result<(), BundleAuthorError> {
    let entries = source
        .iter()
        .map(|entry| BundleEntryInput {
            path: &entry.path,
            contents: &entry.contents,
        })
        .collect::<Vec<_>>();
    let bytes = encode(&entries).map_err(BundleAuthorError::BundleInvalid)?;
    ReleaseBundle::parse(&bytes).map_err(BundleAuthorError::BundleInvalid)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                BundleAuthorError::OutputAlreadyExists
            } else {
                BundleAuthorError::OutputCreationFailed
            }
        })?;
    let mut output_guard = NewOutputGuard::new(output);
    let write_result = file
        .write_all(&bytes)
        .map_err(|_| BundleAuthorError::OutputWriteFailed)
        .and_then(|()| {
            file.sync_all()
                .map_err(|_| BundleAuthorError::OutputSyncFailed)
        });
    drop(file);
    write_result?;
    output_guard.keep();
    Ok(())
}

/// Removes only the output file created by the current unsuccessful call.
struct NewOutputGuard<'path> {
    path: &'path Path,
    keep: bool,
}

impl<'path> NewOutputGuard<'path> {
    fn new(path: &'path Path) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for NewOutputGuard<'_> {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(self.path);
        }
    }
}
