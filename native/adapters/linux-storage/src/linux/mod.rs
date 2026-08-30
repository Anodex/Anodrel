//! Linux state-service composition over private directory descriptors.

mod directories;
mod files;

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anodrel_application::ApplicationIdentity;
use anodrel_paths::ApplicationDirectories;
use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};

use self::{directories::Directory, files::ReadError};

const STATE_FILE_NAME: &str = "state.anodrel.v1";
const BACKUP_FILE_NAME: &str = "state.anodrel.v1.bak";
const STAGING_FILE_NAME: &str = "state.anodrel.v1.stage";
const PRIVATE_COMPONENT_START: usize = 2;

/// Host-owned Linux application-state adapter for one validated identity.
pub struct LinuxStorageService {
    account_home: PathBuf,
    relative_data_directory: PathBuf,
    private_component_start: usize,
    operation_lock: Mutex<()>,
}

impl LinuxStorageService {
    /// Builds one adapter for the current effective account and identity.
    ///
    /// It performs only the bounded account-root lookup. Directory creation and
    /// file access begin when the portable storage operation is called.
    pub fn new(identity: &ApplicationIdentity) -> Result<Self, LinuxStorageInitializationError> {
        let local_data_root = anodrel_linux_paths::local_data_root()
            .map_err(|_| LinuxStorageInitializationError::Unavailable)?;
        let directories = ApplicationDirectories::from_local_data_root(&local_data_root, identity)
            .map_err(|_| LinuxStorageInitializationError::Unavailable)?;
        let account_home = home_from_local_data_root(&local_data_root)
            .ok_or(LinuxStorageInitializationError::Unavailable)?;
        let relative_data_directory = directories
            .data()
            .strip_prefix(&account_home)
            .map_err(|_| LinuxStorageInitializationError::Unavailable)?
            .to_path_buf();
        Self::from_parts(
            account_home,
            relative_data_directory,
            PRIVATE_COMPONENT_START,
        )
    }

    fn from_parts(
        account_home: PathBuf,
        relative_data_directory: PathBuf,
        private_component_start: usize,
    ) -> Result<Self, LinuxStorageInitializationError> {
        if !account_home.is_absolute() || relative_data_directory.is_absolute() {
            return Err(LinuxStorageInitializationError::Unavailable);
        }
        Ok(Self {
            account_home,
            relative_data_directory,
            private_component_start,
            operation_lock: Mutex::new(()),
        })
    }

    fn open_data_directory(&self, create: bool) -> Result<Option<Directory>, StorageServiceError> {
        directories::open_data_directory(
            &self.account_home,
            &self.relative_data_directory,
            self.private_component_start,
            create,
        )
        .map_err(|_| StorageServiceError::Unavailable)
    }

    fn read_snapshot(
        directory: &Directory,
        name: &str,
    ) -> Result<StorageRead, StorageServiceError> {
        let Some(bytes) =
            files::read_regular_file(directory, name, anodrel_storage::MAX_STORAGE_SNAPSHOT_BYTES)
                .map_err(map_read_error)?
        else {
            return Ok(StorageRead::Absent);
        };
        let snapshot =
            String::from_utf8(bytes).map_err(|_| StorageServiceError::StoredSnapshotInvalid)?;
        StorageSnapshot::new(snapshot)
            .map(StorageRead::Snapshot)
            .map_err(|_| StorageServiceError::StoredSnapshotTooLarge)
    }

    #[cfg(test)]
    fn fixture_data_path(&self) -> PathBuf {
        self.account_home.join(&self.relative_data_directory)
    }
}

impl StorageService for LinuxStorageService {
    fn read(&self) -> Result<StorageRead, StorageServiceError> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| StorageServiceError::Unavailable)?;
        let Some(directory) = self.open_data_directory(false)? else {
            return Ok(StorageRead::Absent);
        };
        match Self::read_snapshot(&directory, STATE_FILE_NAME) {
            Ok(StorageRead::Snapshot(snapshot)) => Ok(StorageRead::Snapshot(snapshot)),
            Ok(StorageRead::Absent) => Self::read_snapshot(&directory, BACKUP_FILE_NAME),
            Err(
                primary @ (StorageServiceError::StoredSnapshotInvalid
                | StorageServiceError::StoredSnapshotTooLarge),
            ) => match Self::read_snapshot(&directory, BACKUP_FILE_NAME) {
                Ok(StorageRead::Snapshot(snapshot)) => Ok(StorageRead::Snapshot(snapshot)),
                _ => Err(primary),
            },
            Err(error) => Err(error),
        }
    }

    fn replace(&self, snapshot: &StorageSnapshot) -> Result<(), StorageServiceError> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| StorageServiceError::Unavailable)?;
        let directory = self
            .open_data_directory(true)?
            .ok_or(StorageServiceError::Unavailable)?;
        files::delete_regular_file_if_present(&directory, STAGING_FILE_NAME)
            .map_err(|_| StorageServiceError::Unavailable)?;
        files::write_complete_file(&directory, STAGING_FILE_NAME, snapshot.as_str().as_bytes())
            .map_err(|_| StorageServiceError::Unavailable)?;
        if files::regular_file_exists(&directory, STATE_FILE_NAME)
            .map_err(|_| StorageServiceError::Unavailable)?
        {
            files::move_regular_file(&directory, STATE_FILE_NAME, BACKUP_FILE_NAME)
                .map_err(|_| StorageServiceError::Unavailable)?;
        }
        files::move_regular_file(&directory, STAGING_FILE_NAME, STATE_FILE_NAME)
            .map_err(|_| StorageServiceError::Unavailable)
    }

    fn clear(&self) -> Result<(), StorageServiceError> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| StorageServiceError::Unavailable)?;
        let Some(directory) = self.open_data_directory(false)? else {
            return Ok(());
        };
        let mut changed = false;
        for name in [STATE_FILE_NAME, BACKUP_FILE_NAME, STAGING_FILE_NAME] {
            changed |= files::delete_regular_file_if_present(&directory, name)
                .map_err(|_| StorageServiceError::Unavailable)?;
        }
        if changed {
            directory
                .sync()
                .map_err(|_| StorageServiceError::Unavailable)?;
        }
        Ok(())
    }
}

impl fmt::Debug for LinuxStorageService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LinuxStorageService(..)")
    }
}

/// Closed construction failure for a Linux state adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxStorageInitializationError {
    /// The current account location cannot safely produce a state-store root.
    Unavailable,
}

impl fmt::Display for LinuxStorageInitializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Linux application state location is unavailable")
    }
}

impl std::error::Error for LinuxStorageInitializationError {}

fn home_from_local_data_root(local_data_root: &Path) -> Option<PathBuf> {
    let share = local_data_root.file_name()?;
    let local = local_data_root.parent()?;
    if share != "share" || local.file_name()? != ".local" {
        return None;
    }
    let home = local.parent()?;
    home.is_absolute().then(|| home.to_path_buf())
}

fn map_read_error(error: ReadError) -> StorageServiceError {
    match error {
        ReadError::Unavailable => StorageServiceError::Unavailable,
        ReadError::TooLarge => StorageServiceError::StoredSnapshotTooLarge,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt, symlink},
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use anodrel_application::{ApplicationIdentity, ApplicationManifest};
    use anodrel_storage::{StorageRead, StorageService, StorageServiceError, StorageSnapshot};

    use super::{
        BACKUP_FILE_NAME, LinuxStorageInitializationError, LinuxStorageService, STAGING_FILE_NAME,
        STATE_FILE_NAME, home_from_local_data_root,
    };

    fn identity() -> ApplicationIdentity {
        ApplicationManifest::parse(
            r#"{
                "manifestVersion": { "major": 1, "minor": 0 },
                "applicationId": "org.anodrel.linux-storage-test",
                "displayName": "Linux Storage Test",
                "content": {
                    "format": "anodrel.text.v1",
                    "path": "content/main.txt",
                    "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                }
            }"#,
        )
        .expect("fixture manifest is valid")
        .identity()
        .clone()
    }

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "anodrel-linux-storage-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos()
        ))
    }

    fn service(root: &Path) -> LinuxStorageService {
        fs::create_dir(root).expect("fixture account root is created");
        LinuxStorageService::from_parts(
            root.to_path_buf(),
            PathBuf::from("Anodrel/Applications/org.anodrel.linux-storage-test/data"),
            0,
        )
        .expect("fixture storage location is valid")
    }

    #[test]
    fn derives_the_current_account_service_without_creating_a_directory() {
        assert!(LinuxStorageService::new(&identity()).is_ok());
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
        let data = service.fixture_data_path();
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
        fs::write(service.fixture_data_path().join(STATE_FILE_NAME), [0xFF])
            .expect("invalid current fixture is written");

        assert_eq!(
            service.read(),
            Ok(StorageRead::Snapshot(
                StorageSnapshot::new("first").unwrap()
            ))
        );
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn rejects_symbolic_and_hard_link_state_files() {
        let root = test_root("links");
        let service = service(&root);
        service
            .replace(&StorageSnapshot::new("state").unwrap())
            .expect("replace succeeds");
        let data = service.fixture_data_path();
        let state = data.join(STATE_FILE_NAME);
        fs::remove_file(&state).expect("state file is removable");
        symlink(data.join("missing-target"), &state).expect("symbolic-link fixture is created");
        assert_eq!(service.read(), Err(StorageServiceError::Unavailable));

        fs::remove_file(&state).expect("symbolic-link fixture is removable");
        fs::write(data.join("linked-source"), "state").expect("link source is written");
        fs::hard_link(data.join("linked-source"), &state).expect("hard-link fixture is created");
        assert_eq!(service.read(), Err(StorageServiceError::Unavailable));
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn rejects_a_symbolic_link_or_permissive_directory_in_the_private_tree() {
        let link_root = test_root("directory-link");
        let linked_service = service(&link_root);
        symlink(link_root.join("elsewhere"), link_root.join("Anodrel"))
            .expect("private-tree link fixture is created");
        assert_eq!(
            linked_service.replace(&StorageSnapshot::new("state").unwrap()),
            Err(StorageServiceError::Unavailable)
        );
        fs::remove_dir_all(link_root).expect("linked fixture tree is removable");

        let permissive_root = test_root("directory-permissions");
        let permissive_service = service(&permissive_root);
        let anodrel = permissive_root.join("Anodrel");
        fs::create_dir(&anodrel).expect("private-tree directory fixture is created");
        fs::set_permissions(&anodrel, fs::Permissions::from_mode(0o755))
            .expect("permissive fixture mode is set");
        assert_eq!(
            permissive_service.replace(&StorageSnapshot::new("state").unwrap()),
            Err(StorageServiceError::Unavailable)
        );
        fs::remove_dir_all(permissive_root).expect("permissive fixture tree is removable");
    }

    #[test]
    fn created_state_tree_and_file_are_private_to_the_effective_account() {
        let root = test_root("permissions");
        let service = service(&root);
        service
            .replace(&StorageSnapshot::new("state").unwrap())
            .expect("replace succeeds");
        let data = service.fixture_data_path();
        let state = fs::metadata(data.join(STATE_FILE_NAME)).expect("state metadata is readable");
        let directory = fs::metadata(&data).expect("directory metadata is readable");
        assert_eq!(state.permissions().mode() & 0o777, 0o600);
        assert_eq!(directory.permissions().mode() & 0o777, 0o700);
        assert_eq!(state.uid(), super::directories::effective_uid());
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn rejects_a_state_file_over_the_portable_snapshot_limit() {
        let root = test_root("oversized");
        let service = service(&root);
        service
            .replace(&StorageSnapshot::new("state").unwrap())
            .expect("replace succeeds");
        fs::write(
            service.fixture_data_path().join(STATE_FILE_NAME),
            vec![b'x'; anodrel_storage::MAX_STORAGE_SNAPSHOT_BYTES + 1],
        )
        .expect("oversized fixture is written");
        assert_eq!(
            service.read(),
            Err(StorageServiceError::StoredSnapshotTooLarge)
        );
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn current_user_root_has_the_fixed_linux_shape() {
        assert_eq!(
            home_from_local_data_root(Path::new("/home/anodrel/.local/share")),
            Some(PathBuf::from("/home/anodrel"))
        );
        assert_eq!(
            home_from_local_data_root(Path::new("/home/anodrel/share")),
            None
        );
        assert!(matches!(
            LinuxStorageService::from_parts(PathBuf::from("relative"), PathBuf::from("data"), 0),
            Err(LinuxStorageInitializationError::Unavailable)
        ));
    }

    #[test]
    fn debug_output_never_reveals_the_account_storage_path() {
        let service = service(&test_root("debug"));
        assert_eq!(format!("{service:?}"), "LinuxStorageService(..)");
    }
}
