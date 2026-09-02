//! Streaming composition of a prepared update candidate and fresh private file.

use std::path::Path;

use anodrel_windows_http::{WindowsHttpsError, get_https};

use crate::{
    DownloadedInstaller, PreparedUpdateDownload, UpdateDownloadError, file::FreshInstallerFile,
};

/// Streams one prepared update image into an updater-owned fresh private file.
///
/// `cache_parent` must be selected by native updater composition, never by an
/// application, protocol message, command line, environment variable, or UI.
/// The returned image matches its signed descriptor but still needs later
/// Authenticode and installer-gate verification before installation.
pub fn download_prepared_update(
    prepared: &PreparedUpdateDownload,
    cache_parent: &Path,
) -> Result<DownloadedInstaller, UpdateDownloadError> {
    let installer = prepared.installer();
    let maximum_bytes =
        usize::try_from(installer.byte_length()).map_err(|_| UpdateDownloadError::ImageMismatch)?;
    let mut output = FreshInstallerFile::create(cache_parent, installer.byte_length())?;
    let mut write_failure = None;
    let transfer = get_https(
        installer.origin(),
        installer.request_path(),
        Some(200),
        maximum_bytes,
        &mut |chunk| match output.write_chunk(chunk) {
            Ok(()) => Ok(()),
            Err(error) => {
                write_failure = Some(error);
                Err(())
            }
        },
    );
    match transfer {
        Ok(_) => output.finish(installer),
        Err(WindowsHttpsError::ConsumerRejected) => Err(write_failure.unwrap_or(
            UpdateDownloadError::TransferFailed(WindowsHttpsError::ConsumerRejected),
        )),
        Err(error) => Err(UpdateDownloadError::TransferFailed(error)),
    }
}
