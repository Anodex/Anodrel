//! Fresh private-file lifecycle for one streamed update image.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anodrel_application::sha256::Sha256Digest;
use anodrel_update_catalogue::UpdateInstaller;

use crate::UpdateDownloadError;

const FILE_ATTEMPTS: usize = 32;
static NEXT_CACHE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// One newly created private file accepting bounded update-image chunks.
pub(crate) struct FreshInstallerFile {
    path: PathBuf,
    file: File,
    hash: Sha256Digest,
    byte_length: u64,
    maximum_bytes: u64,
    retain: bool,
}

impl FreshInstallerFile {
    pub(crate) fn create(
        cache_parent: &Path,
        maximum_bytes: u64,
    ) -> Result<Self, UpdateDownloadError> {
        let parent = canonical_cache_parent(cache_parent)?;
        for _ in 0..FILE_ATTEMPTS {
            let sequence = NEXT_CACHE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".anodrel-update-{}-{sequence}.exe",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok(Self::new(path, file, maximum_bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(UpdateDownloadError::CacheFileCreationFailed),
            }
        }
        Err(UpdateDownloadError::CacheFileCreationFailed)
    }

    fn new(path: PathBuf, file: File, maximum_bytes: u64) -> Self {
        Self {
            path,
            file,
            hash: Sha256Digest::new(),
            byte_length: 0,
            maximum_bytes,
            retain: false,
        }
    }

    pub(crate) fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), UpdateDownloadError> {
        let new_length = self
            .byte_length
            .checked_add(chunk.len() as u64)
            .filter(|length| *length <= self.maximum_bytes)
            .ok_or(UpdateDownloadError::ImageMismatch)?;
        self.file
            .write_all(chunk)
            .map_err(|_| UpdateDownloadError::CacheFileWriteFailed)?;
        self.hash.update(chunk);
        self.byte_length = new_length;
        Ok(())
    }

    pub(crate) fn finish(
        mut self,
        installer: &UpdateInstaller,
    ) -> Result<DownloadedInstaller, UpdateDownloadError> {
        self.file
            .sync_all()
            .map_err(|_| UpdateDownloadError::CacheFileSyncFailed)?;
        let digest = std::mem::take(&mut self.hash).finish();
        if !installer.matches_descriptor(self.byte_length, digest) {
            return Err(UpdateDownloadError::ImageMismatch);
        }
        self.retain = true;
        Ok(DownloadedInstaller {
            path: self.path.clone(),
            cleanup_on_drop: true,
        })
    }

    #[cfg(test)]
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FreshInstallerFile {
    fn drop(&mut self) {
        if !self.retain {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// One private update image that passed its signed byte descriptor.
///
/// Its path remains host-only data and the file is removed when this value
/// drops. Later installer handoff must independently verify Authenticode and
/// the embedded release before using the file.
pub struct DownloadedInstaller {
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl DownloadedInstaller {
    /// Returns the private absolute image path for a later native installer gate.
    ///
    /// This must never be serialized to an application, renderer, or protocol
    /// response.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn retain_for_recovery(&mut self) {
        self.cleanup_on_drop = false;
    }
}

impl fmt::Debug for DownloadedInstaller {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DownloadedInstaller(..)")
    }
}

impl Drop for DownloadedInstaller {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn canonical_cache_parent(path: &Path) -> Result<PathBuf, UpdateDownloadError> {
    if !path.is_absolute() {
        return Err(UpdateDownloadError::CacheParentInvalid);
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| UpdateDownloadError::CacheParentInvalid)?;
    if !metadata.is_dir() || is_link_like(&metadata) {
        return Err(UpdateDownloadError::CacheParentInvalid);
    }
    fs::canonicalize(path).map_err(|_| UpdateDownloadError::CacheParentInvalid)
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

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use anodrel_application::sha256;
    use anodrel_update_catalogue::UpdateCatalogue;

    use super::FreshInstallerFile;
    use crate::UpdateDownloadError;

    #[test]
    fn checked_image_file_is_removed_after_its_host_handle_drops() {
        let parent = TemporaryDirectory::new();
        let image = b"checked update image";
        let catalogue = catalogue(image);
        let mut fresh = FreshInstallerFile::create(parent.path(), image.len() as u64)
            .expect("fresh cache file is created");
        let path = fresh.path().to_path_buf();
        fresh.write_chunk(image).expect("checked bytes write");
        let downloaded = fresh
            .finish(catalogue.installer())
            .expect("exact bytes become a downloaded installer");
        assert_eq!(
            std::fs::read(downloaded.path()).expect("image is readable"),
            image
        );
        drop(downloaded);
        assert!(!path.exists());
    }

    #[test]
    fn mismatched_bytes_remove_the_new_file_without_retaining_a_candidate() {
        let parent = TemporaryDirectory::new();
        let catalogue = catalogue(b"expected image");
        let mut fresh =
            FreshInstallerFile::create(parent.path(), 32).expect("fresh cache file is created");
        let path = fresh.path().to_path_buf();
        fresh.write_chunk(b"substituted").expect("bytes write");
        assert!(matches!(
            fresh.finish(catalogue.installer()),
            Err(UpdateDownloadError::ImageMismatch)
        ));
        assert!(!path.exists());
    }

    #[test]
    fn a_relative_cache_parent_is_rejected_before_file_creation() {
        assert!(matches!(
            FreshInstallerFile::create(Path::new("relative-cache"), 1),
            Err(UpdateDownloadError::CacheParentInvalid)
        ));
    }

    fn catalogue(image: &[u8]) -> UpdateCatalogue {
        let digest = sha256::to_lower_hex(&sha256::digest(image));
        UpdateCatalogue::parse(&format!(
            r#"{{
  "formatVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.update-download-test",
  "packageVersion": {{ "major": 1, "minor": 2, "patch": 3 }},
  "publisher": {{ "leafCertificateSha256": "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585" }},
  "installer": {{
    "origin": {{ "host": "updates.example.test", "port": 443 }},
    "path": "/anodrel/1.2.3/installer.exe",
    "byteLength": {},
    "sha256": "{digest}"
  }}
}}"#,
            image.len()
        ))
        .expect("fixture catalogue is valid")
    }

    static NEXT_TEMPORARY_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new() -> Self {
            for _ in 0..32 {
                let sequence = NEXT_TEMPORARY_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "anodrel-update-download-test-{}-{sequence}",
                    std::process::id()
                ));
                if std::fs::create_dir(&path).is_ok() {
                    return Self(path);
                }
            }
            panic!("a temporary update cache directory could not be created");
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
