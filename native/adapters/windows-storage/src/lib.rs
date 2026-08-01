#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows adapter for one bounded application-state snapshot.
//!
//! The adapter receives only host-derived application directories. It creates
//! the fixed state location, stages complete replacements, and retains one
//! previous committed file as a recovery candidate. It never exposes a path,
//! handle, or stored value outside the portable storage boundary.

mod raw;

use std::{fmt, path::PathBuf, sync::Mutex};

use anodrel_paths::ApplicationDirectories;
use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};

const STATE_FILE_NAME: &str = "state.anodrel.v1";
const BACKUP_FILE_NAME: &str = "state.anodrel.v1.bak";
const STAGING_FILE_NAME: &str = "state.anodrel.v1.stage";

/// Host-owned Windows application-state adapter for one validated identity.
pub struct WindowsStorageService {
    data_directory: PathBuf,
    state_path: PathBuf,
    backup_path: PathBuf,
    staging_path: PathBuf,
    operation_lock: Mutex<()>,
}

impl WindowsStorageService {
    /// Builds one adapter from already host-derived application directories.
    #[must_use]
    pub fn new(directories: &ApplicationDirectories) -> Self {
        let data_directory = directories.data().to_path_buf();
        Self {
            state_path: data_directory.join(STATE_FILE_NAME),
            backup_path: data_directory.join(BACKUP_FILE_NAME),
            staging_path: data_directory.join(STAGING_FILE_NAME),
            data_directory,
            operation_lock: Mutex::new(()),
        }
    }

    fn read_snapshot(&self, path: &std::path::Path) -> Result<StorageRead, StorageServiceError> {
        let Some(bytes) = raw::read_regular_file(path, anodrel_storage::MAX_STORAGE_SNAPSHOT_BYTES)
            .map_err(map_read_error)?
        else {
            return Ok(StorageRead::Absent);
        };
        let text =
            String::from_utf8(bytes).map_err(|_| StorageServiceError::StoredSnapshotInvalid)?;
        StorageSnapshot::new(text)
            .map(StorageRead::Snapshot)
            .map_err(|_| StorageServiceError::StoredSnapshotTooLarge)
    }

    fn ensure_data_directory(&self) -> Result<(), StorageServiceError> {
        raw::ensure_directory_tree(&self.data_directory)
            .map_err(|_| StorageServiceError::Unavailable)
    }
}

impl StorageService for WindowsStorageService {
    fn read(&self) -> Result<StorageRead, StorageServiceError> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| StorageServiceError::Unavailable)?;
        match self.read_snapshot(&self.state_path) {
            Ok(StorageRead::Snapshot(snapshot)) => Ok(StorageRead::Snapshot(snapshot)),
            Ok(StorageRead::Absent) => self.read_snapshot(&self.backup_path),
            Err(
                primary_error @ (StorageServiceError::StoredSnapshotInvalid
                | StorageServiceError::StoredSnapshotTooLarge),
            ) => match self.read_snapshot(&self.backup_path) {
                Ok(StorageRead::Snapshot(snapshot)) => Ok(StorageRead::Snapshot(snapshot)),
                _ => Err(primary_error),
            },
            Err(error) => Err(error),
        }
    }

    fn replace(&self, snapshot: &StorageSnapshot) -> Result<(), StorageServiceError> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| StorageServiceError::Unavailable)?;
        self.ensure_data_directory()?;
        raw::delete_regular_file_if_present(&self.staging_path)
            .map_err(|_| StorageServiceError::Unavailable)?;
        raw::write_complete_file(&self.staging_path, snapshot.as_str().as_bytes())
            .map_err(|_| StorageServiceError::Unavailable)?;

        if raw::regular_file_exists(&self.state_path)
            .map_err(|_| StorageServiceError::Unavailable)?
        {
            raw::move_file(&self.state_path, &self.backup_path)
                .map_err(|_| StorageServiceError::Unavailable)?;
        }
        raw::move_file(&self.staging_path, &self.state_path)
            .map_err(|_| StorageServiceError::Unavailable)
    }

    fn clear(&self) -> Result<(), StorageServiceError> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| StorageServiceError::Unavailable)?;
        for path in [&self.state_path, &self.backup_path, &self.staging_path] {
            raw::delete_regular_file_if_present(path)
                .map_err(|_| StorageServiceError::Unavailable)?;
        }
        Ok(())
    }
}

impl fmt::Debug for WindowsStorageService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("WindowsStorageService(..)")
    }
}

fn map_read_error(error: raw::ReadError) -> StorageServiceError {
    match error {
        raw::ReadError::Unavailable => StorageServiceError::Unavailable,
        raw::ReadError::TooLarge => StorageServiceError::StoredSnapshotTooLarge,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use anodrel_application::ApplicationManifest;
    use anodrel_paths::ApplicationDirectories;
    use anodrel_storage::{StorageRead, StorageService, StorageSnapshot};

    use super::{BACKUP_FILE_NAME, STAGING_FILE_NAME, STATE_FILE_NAME, WindowsStorageService};

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "anodrel-windows-storage-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos()
        ))
    }

    fn service(root: &std::path::Path) -> WindowsStorageService {
        let manifest = ApplicationManifest::parse(r#"{
            "manifestVersion":{"major":1,"minor":0},
            "applicationId":"org.anodrel.storage-test",
            "displayName":"Storage Test",
            "content":{"format":"anodrel.text.v1","path":"content/main.txt","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}
        }"#).expect("fixture manifest is valid");
        let directories = ApplicationDirectories::from_local_data_root(root, manifest.identity())
            .expect("fixture paths are valid");
        WindowsStorageService::new(&directories)
    }

    #[test]
    fn reads_absent_state_then_replaces_and_clears_one_complete_snapshot() {
        let root = test_root("round-trip");
        let service = service(&root);
        assert_eq!(service.read(), Ok(StorageRead::Absent));

        service
            .replace(&StorageSnapshot::new("saved state").unwrap())
            .expect("replace succeeds");
        assert_eq!(
            service.read(),
            Ok(StorageRead::Snapshot(
                StorageSnapshot::new("saved state").unwrap()
            ))
        );

        service.clear().expect("clear succeeds");
        assert_eq!(service.read(), Ok(StorageRead::Absent));
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn falls_back_to_the_complete_backup_after_an_interrupted_replacement() {
        let root = test_root("recovery");
        let service = service(&root);
        service
            .replace(&StorageSnapshot::new("first").unwrap())
            .expect("first replace succeeds");
        let data = root.join("Anodrel\\Applications\\org.anodrel.storage-test\\data");
        fs::rename(data.join(STATE_FILE_NAME), data.join(BACKUP_FILE_NAME))
            .expect("prior state is retained as backup");
        fs::write(data.join(STAGING_FILE_NAME), "incomplete staging data")
            .expect("staging fixture is written");

        assert_eq!(
            service.read(),
            Ok(StorageRead::Snapshot(
                StorageSnapshot::new("first").unwrap()
            ))
        );
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn uses_a_complete_backup_when_the_current_file_is_not_valid_utf8() {
        let root = test_root("invalid-current");
        let service = service(&root);
        service
            .replace(&StorageSnapshot::new("first").unwrap())
            .expect("first replace succeeds");
        service
            .replace(&StorageSnapshot::new("second").unwrap())
            .expect("second replace succeeds");
        let data = root.join("Anodrel\\Applications\\org.anodrel.storage-test\\data");
        fs::write(data.join(STATE_FILE_NAME), [0xFF]).expect("invalid current fixture is written");

        assert_eq!(
            service.read(),
            Ok(StorageRead::Snapshot(
                StorageSnapshot::new("first").unwrap()
            ))
        );
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn rejects_a_directory_in_place_of_the_fixed_state_file() {
        let root = test_root("directory");
        let service = service(&root);
        service
            .replace(&StorageSnapshot::new("state").unwrap())
            .expect("replace creates the data directory");
        let data = root.join("Anodrel\\Applications\\org.anodrel.storage-test\\data");
        fs::remove_file(data.join(STATE_FILE_NAME)).expect("current file is removable");
        fs::create_dir(data.join(STATE_FILE_NAME)).expect("directory fixture is created");

        assert_eq!(
            service.read(),
            Err(anodrel_storage::StorageServiceError::Unavailable)
        );
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn debug_output_does_not_reveal_host_storage_paths() {
        let service = service(&test_root("debug"));
        assert_eq!(format!("{service:?}"), "WindowsStorageService(..)");
    }
}
