//! Fixed staging of the current signed installer for later product removal.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anodrel_application::sha256::{self, Sha256Digest};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::{MAX_PAYLOAD_BYTES, ReleaseManifest, staging::StagedRelease};

const UNINSTALLER_DIRECTORY: &str = "uninstaller";
const UNINSTALLER_IMAGE_NAME: &str = "anodrel-windows-installer.exe";
const COPY_BUFFER_BYTES: usize = 16 * 1024;
const MAX_IMAGE_OVERHEAD_BYTES: u64 = 32 * 1024 * 1024;
const MAX_INSTALLED_UNINSTALLER_BYTES: u64 = MAX_PAYLOAD_BYTES + MAX_IMAGE_OVERHEAD_BYTES;

/// A current installer image could not become the fixed staged uninstaller.
#[derive(Debug)]
pub enum InstalledUninstallerError {
    /// The current installer executable could not be located or read safely.
    CurrentImageUnavailable,
    /// The current installer image exceeded the fixed installed-image bound.
    CurrentImageTooLarge,
    /// The installer-owned staged uninstaller location could not be created safely.
    DestinationUnavailable,
    /// Streaming the current image to its new fixed location failed.
    ImageCopyFailed,
    /// The persisted image did not retain the bytes read from the current image.
    ImageVerificationFailed,
    /// Windows did not accept the copied installer image's Authenticode signature.
    SignatureInvalid(SignatureError),
    /// The copied image signer differed from the current release publisher.
    PublisherMismatch,
}

impl fmt::Display for InstalledUninstallerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CurrentImageUnavailable => "the current installer image is unavailable",
            Self::CurrentImageTooLarge => "the current installer image exceeds the fixed limit",
            Self::DestinationUnavailable => "the installed uninstaller location is unavailable",
            Self::ImageCopyFailed => "the installed uninstaller image could not be copied",
            Self::ImageVerificationFailed => "the installed uninstaller image did not verify",
            Self::SignatureInvalid(_) => {
                "Windows did not accept the installed uninstaller signature"
            }
            Self::PublisherMismatch => {
                "the installed uninstaller publisher does not match the release"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for InstalledUninstallerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SignatureInvalid(error) => Some(error),
            Self::CurrentImageUnavailable
            | Self::CurrentImageTooLarge
            | Self::DestinationUnavailable
            | Self::ImageCopyFailed
            | Self::ImageVerificationFailed
            | Self::PublisherMismatch => None,
        }
    }
}

/// Copies and verifies the current signed installer under one private stage.
///
/// The destination is entirely installer-derived and the caller has already
/// activated the current signed release. This function accepts no path,
/// command, application value, or release metadata from a caller. It does not
/// promote the stage, publish policy, create registration, elevate, or launch
/// a process.
pub(crate) fn stage_current_installer_image(
    staged: &StagedRelease,
    manifest: &ReleaseManifest,
) -> Result<(), InstalledUninstallerError> {
    let source =
        std::env::current_exe().map_err(|_| InstalledUninstallerError::CurrentImageUnavailable)?;
    let destination = staged_uninstaller_path(staged.package_root())?;
    copy_current_image(&source, &destination)?;
    let signer = verify_embedded_signature(&destination)
        .map_err(InstalledUninstallerError::SignatureInvalid)?;
    manifest
        .matches_publisher_fingerprint(signer.as_bytes())
        .then_some(())
        .ok_or(InstalledUninstallerError::PublisherMismatch)
}

/// Derives the sole installer-controlled uninstaller path below a package root.
pub(crate) fn installed_uninstaller_path(package_root: &Path) -> PathBuf {
    package_root
        .join(UNINSTALLER_DIRECTORY)
        .join(UNINSTALLER_IMAGE_NAME)
}

fn staged_uninstaller_path(package_root: &Path) -> Result<PathBuf, InstalledUninstallerError> {
    let destination = installed_uninstaller_path(package_root);
    let directory = destination
        .parent()
        .ok_or(InstalledUninstallerError::DestinationUnavailable)?;
    crate::staging::create_private_directory(package_root, directory)
        .map_err(|_| InstalledUninstallerError::DestinationUnavailable)?;
    Ok(destination)
}

fn copy_current_image(source: &Path, destination: &Path) -> Result<(), InstalledUninstallerError> {
    let source_metadata =
        fs::metadata(source).map_err(|_| InstalledUninstallerError::CurrentImageUnavailable)?;
    if !source_metadata.is_file() {
        return Err(InstalledUninstallerError::CurrentImageUnavailable);
    }
    let source_length = source_metadata.len();
    if source_length > MAX_INSTALLED_UNINSTALLER_BYTES {
        return Err(InstalledUninstallerError::CurrentImageTooLarge);
    }
    let maximum = usize::try_from(source_length)
        .map_err(|_| InstalledUninstallerError::CurrentImageTooLarge)?;
    let mut input =
        File::open(source).map_err(|_| InstalledUninstallerError::CurrentImageUnavailable)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| InstalledUninstallerError::DestinationUnavailable)?;
    let source_digest = copy_and_digest(&mut input, &mut output)?;
    output
        .sync_all()
        .map_err(|_| InstalledUninstallerError::ImageCopyFailed)?;
    drop(output);

    let mut persisted =
        File::open(destination).map_err(|_| InstalledUninstallerError::ImageVerificationFailed)?;
    let Some((persisted_digest, persisted_length)) =
        sha256::digest_reader_limited(&mut persisted, maximum)
            .map_err(|_| InstalledUninstallerError::ImageVerificationFailed)?
    else {
        return Err(InstalledUninstallerError::ImageVerificationFailed);
    };
    (persisted_length == maximum && persisted_digest == source_digest)
        .then_some(())
        .ok_or(InstalledUninstallerError::ImageVerificationFailed)
}

fn copy_and_digest(
    input: &mut File,
    output: &mut File,
) -> Result<[u8; 32], InstalledUninstallerError> {
    let mut buffer = [0_u8; COPY_BUFFER_BYTES];
    let mut digest = Sha256Digest::new();
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|_| InstalledUninstallerError::ImageCopyFailed)?;
        if read == 0 {
            return Ok(digest.finish());
        }
        output
            .write_all(&buffer[..read])
            .map_err(|_| InstalledUninstallerError::ImageCopyFailed)?;
        digest.update(&buffer[..read]);
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{InstalledUninstallerError, copy_current_image, installed_uninstaller_path};
    use crate::test_support::TestDirectory;

    #[test]
    fn fixed_uninstaller_path_is_not_derived_from_product_text() {
        assert_eq!(
            installed_uninstaller_path(std::path::Path::new("C:\\Program Files\\Anodrel\\1.2.3")),
            PathBuf::from(
                "C:\\Program Files\\Anodrel\\1.2.3\\uninstaller\\anodrel-windows-installer.exe"
            )
        );
    }

    #[test]
    fn persisted_copy_must_match_the_streamed_current_image_bytes() {
        let directory = TestDirectory::new("installed-uninstaller");
        let source = directory.path().join("source.exe");
        let destination = directory.path().join("destination.exe");
        fs::write(&source, b"fixed signed image bytes").expect("source image is written");

        copy_current_image(&source, &destination).expect("copy remains exact");

        assert_eq!(
            fs::read(destination).expect("destination image is readable"),
            b"fixed signed image bytes"
        );
    }

    #[test]
    fn existing_destination_is_never_replaced() {
        let directory = TestDirectory::new("installed-uninstaller");
        let source = directory.path().join("source.exe");
        let destination = directory.path().join("destination.exe");
        fs::write(&source, b"new image").expect("source image is written");
        fs::write(&destination, b"existing image").expect("destination image is written");

        assert!(matches!(
            copy_current_image(&source, &destination),
            Err(InstalledUninstallerError::DestinationUnavailable)
        ));
        assert_eq!(
            fs::read(destination).expect("existing image remains readable"),
            b"existing image"
        );
    }
}
