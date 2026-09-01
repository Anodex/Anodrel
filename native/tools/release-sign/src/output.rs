//! Fresh bounded release-image copying for the signer.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::ReleaseSignError;

const MAX_RELEASE_IMAGE_BYTES: u64 = 576 * 1024 * 1024;

/// A release-image copy that this invocation may remove until it is retained.
pub(super) struct FreshReleaseImage {
    path: PathBuf,
    keep: bool,
}

impl FreshReleaseImage {
    /// Copies one checked release image to one absent output file.
    pub(super) fn copy_from(input: &Path, output: &Path) -> Result<Self, ReleaseSignError> {
        validate_output(output)?;
        let input_metadata = fs::metadata(input).map_err(|_| ReleaseSignError::CopyFailed)?;
        if !input_metadata.is_file() {
            return Err(ReleaseSignError::CopyFailed);
        }
        if input_metadata.len() > MAX_RELEASE_IMAGE_BYTES {
            return Err(ReleaseSignError::InputTooLarge);
        }
        let mut input_file = File::open(input).map_err(|_| ReleaseSignError::CopyFailed)?;
        let mut output_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    ReleaseSignError::OutputAlreadyExists
                } else {
                    ReleaseSignError::CopyFailed
                }
            })?;
        let fresh = Self {
            path: output.to_path_buf(),
            keep: false,
        };
        let copy_result = copy_bounded(&mut input_file, &mut output_file).and_then(|()| {
            output_file
                .sync_all()
                .map_err(|_| ReleaseSignError::CopyFailed)
        });
        drop(output_file);
        copy_result?;
        Ok(fresh)
    }

    /// Returns the exclusive fresh output path.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Retains the new output after every signing check has passed.
    pub(super) fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for FreshReleaseImage {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn validate_output(output: &Path) -> Result<(), ReleaseSignError> {
    if !output.is_absolute() {
        return Err(ReleaseSignError::OutputInvalid);
    }
    if output
        .try_exists()
        .map_err(|_| ReleaseSignError::OutputInvalid)?
    {
        return Err(ReleaseSignError::OutputAlreadyExists);
    }
    let parent = output.parent().ok_or(ReleaseSignError::OutputInvalid)?;
    let parent_metadata =
        fs::symlink_metadata(parent).map_err(|_| ReleaseSignError::OutputInvalid)?;
    if !parent_metadata.is_dir() || is_link_like(&parent_metadata) {
        return Err(ReleaseSignError::OutputInvalid);
    }
    Ok(())
}

fn copy_bounded(input: &mut File, output: &mut File) -> Result<(), ReleaseSignError> {
    let mut buffer = [0_u8; 64 * 1024];
    let mut copied = 0_u64;
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|_| ReleaseSignError::CopyFailed)?;
        if read == 0 {
            return Ok(());
        }
        copied = copied
            .checked_add(read as u64)
            .ok_or(ReleaseSignError::InputTooLarge)?;
        if copied > MAX_RELEASE_IMAGE_BYTES {
            return Err(ReleaseSignError::InputTooLarge);
        }
        output
            .write_all(&buffer[..read])
            .map_err(|_| ReleaseSignError::CopyFailed)?;
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
