//! Linux crash-store composition over private host directories.

mod directories;
mod files;

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anodrel_crash::{CrashRecord, CrashReportError, CrashReporter, serialize};
use anodrel_paths::HostDirectories;

/// Records retained before best-effort eviction removes the oldest.
pub const RETAINED_RECORDS: usize = 8;

const FILE_PREFIX: &str = "crash-";
const FILE_SUFFIX: &str = ".anodrel.v1";
const PRIVATE_COMPONENT_START: usize = 2;

/// Host-owned Linux crash-record store.
pub struct LinuxCrashStore {
    account_home: PathBuf,
    relative_logs_directory: PathBuf,
    private_component_start: usize,
    operation_lock: Mutex<()>,
}

impl LinuxCrashStore {
    /// Builds one store for the current effective account.
    ///
    /// It performs only the bounded account-root lookup. The first report owns
    /// directory creation and file operations.
    pub fn new() -> Result<Self, LinuxCrashInitializationError> {
        let local_data_root = anodrel_linux_paths::local_data_root()
            .map_err(|_| LinuxCrashInitializationError::Unavailable)?;
        let directories = HostDirectories::from_local_data_root(&local_data_root)
            .map_err(|_| LinuxCrashInitializationError::Unavailable)?;
        let account_home = home_from_local_data_root(&local_data_root)
            .ok_or(LinuxCrashInitializationError::Unavailable)?;
        let relative_logs_directory = directories
            .logs()
            .strip_prefix(&account_home)
            .map_err(|_| LinuxCrashInitializationError::Unavailable)?
            .to_path_buf();
        Self::from_parts(
            account_home,
            relative_logs_directory,
            PRIVATE_COMPONENT_START,
        )
    }

    fn from_parts(
        account_home: PathBuf,
        relative_logs_directory: PathBuf,
        private_component_start: usize,
    ) -> Result<Self, LinuxCrashInitializationError> {
        if !account_home.is_absolute() || relative_logs_directory.is_absolute() {
            return Err(LinuxCrashInitializationError::Unavailable);
        }
        Ok(Self {
            account_home,
            relative_logs_directory,
            private_component_start,
            operation_lock: Mutex::new(()),
        })
    }

    fn open_logs_directory(&self) -> Result<directories::Directory, CrashReportError> {
        directories::open_host_logs(
            &self.account_home,
            &self.relative_logs_directory,
            self.private_component_start,
        )
        .map_err(|_| CrashReportError::LocationUnavailable)
    }

    fn stored_sequences(directory: &directories::Directory) -> Result<Vec<u64>, CrashReportError> {
        let mut sequences: Vec<u64> = files::private_record_names(directory)
            .map_err(|_| CrashReportError::WriteFailed)?
            .iter()
            .filter_map(|name| sequence_of(name))
            .collect();
        sequences.sort_unstable();
        Ok(sequences)
    }

    fn evict_oldest(&self, directory: &directories::Directory, sequences: &[u64]) {
        let excess = sequences.len().saturating_sub(RETAINED_RECORDS);
        let mut changed = false;
        for sequence in &sequences[..excess] {
            changed |=
                files::delete_private_record(directory, &file_name(*sequence)).unwrap_or(false);
        }
        if changed {
            let _ = directory.sync();
        }
    }

    #[cfg(test)]
    fn fixture_logs_path(&self) -> PathBuf {
        self.account_home.join(&self.relative_logs_directory)
    }
}

impl CrashReporter for LinuxCrashStore {
    fn report(&self, record: &CrashRecord) -> Result<u64, CrashReportError> {
        let _guard = self
            .operation_lock
            .try_lock()
            .map_err(|_| CrashReportError::WriteFailed)?;
        let directory = self.open_logs_directory()?;
        let mut sequences = Self::stored_sequences(&directory)?;
        let sequence = sequences
            .last()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(CrashReportError::WriteFailed)?;
        let text = serialize(record, sequence)?;
        files::write_new_record(&directory, &file_name(sequence), text.as_bytes())
            .map_err(|_| CrashReportError::WriteFailed)?;
        directory
            .sync()
            .map_err(|_| CrashReportError::WriteFailed)?;

        sequences.push(sequence);
        self.evict_oldest(&directory, &sequences);
        Ok(sequence)
    }
}

impl fmt::Debug for LinuxCrashStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LinuxCrashStore(..)")
    }
}

/// Closed construction failure for a Linux crash-record store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxCrashInitializationError {
    /// The current account location cannot safely produce a host log root.
    Unavailable,
}

impl fmt::Display for LinuxCrashInitializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Linux crash-record location is unavailable")
    }
}

impl std::error::Error for LinuxCrashInitializationError {}

fn home_from_local_data_root(local_data_root: &Path) -> Option<PathBuf> {
    let share = local_data_root.file_name()?;
    let local = local_data_root.parent()?;
    if share != "share" || local.file_name()? != ".local" {
        return None;
    }
    let home = local.parent()?;
    home.is_absolute().then(|| home.to_path_buf())
}

fn file_name(sequence: u64) -> String {
    format!("{FILE_PREFIX}{sequence}{FILE_SUFFIX}")
}

fn sequence_of(name: &str) -> Option<u64> {
    let sequence = name.strip_prefix(FILE_PREFIX)?.strip_suffix(FILE_SUFFIX)?;
    if sequence.is_empty()
        || (sequence.len() > 1 && sequence.starts_with('0'))
        || !sequence.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    sequence.parse().ok().filter(|sequence: &u64| *sequence > 0)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use anodrel_crash::{CrashRecord, CrashReportError, CrashReporter, CrashSite, CrashSurface};

    use super::{
        LinuxCrashInitializationError, LinuxCrashStore, RETAINED_RECORDS, file_name,
        home_from_local_data_root, sequence_of,
    };

    fn record() -> CrashRecord {
        CrashRecord::new(
            CrashSite::WindowProcedure,
            CrashSurface::StartupLab,
            "0.1.0",
        )
    }

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "anodrel-linux-crash-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos()
        ))
    }

    fn store(root: &Path) -> LinuxCrashStore {
        if !root.exists() {
            fs::create_dir(root).expect("fixture account root is created");
        }
        LinuxCrashStore::from_parts(root.to_path_buf(), PathBuf::from("Anodrel/Host/logs"), 0)
            .expect("fixture store is valid")
    }

    #[test]
    fn derives_the_current_account_store_without_creating_a_directory() {
        assert!(LinuxCrashStore::new().is_ok());
    }

    #[test]
    fn first_report_creates_one_private_record() {
        let root = test_root("first");
        let store = store(&root);
        assert_eq!(store.report(&record()), Ok(1));
        let logs = store.fixture_logs_path();
        let written = fs::read_to_string(logs.join(file_name(1))).expect("record is written");
        assert_eq!(
            written,
            "format=anodrel.crash.v1\n\
             site=window-procedure\n\
             surface=startup-lab\n\
             hostVersion=0.1.0\n\
             sequence=1\n"
        );
        let mode = fs::metadata(logs.join(file_name(1)))
            .expect("record metadata is available")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn a_later_store_continues_the_sequence_without_colliding() {
        let root = test_root("sequence");
        for expected in 1..=3 {
            let store = store(&root);
            assert_eq!(store.report(&record()), Ok(expected));
        }
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn retention_keeps_only_the_newest_records() {
        let root = test_root("retention");
        let store = store(&root);
        let total = RETAINED_RECORDS as u64 + 3;
        for sequence in 1..=total {
            assert_eq!(store.report(&record()), Ok(sequence));
        }
        let logs = store.fixture_logs_path();
        let remaining: Vec<u64> = (total - RETAINED_RECORDS as u64 + 1..=total).collect();
        for sequence in 1..=total {
            assert_eq!(
                logs.join(file_name(sequence)).exists(),
                remaining.contains(&sequence)
            );
        }
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn unrelated_and_linked_files_are_ignored_not_removed() {
        let root = test_root("ignored");
        let store = store(&root);
        assert_eq!(store.report(&record()), Ok(1));
        let logs = store.fixture_logs_path();
        let stray = logs.join("notes.txt");
        fs::write(&stray, "not a record").expect("stray file is written");
        symlink(logs.join("missing"), logs.join(file_name(9))).expect("link fixture is created");
        assert_eq!(store.report(&record()), Ok(2));
        assert!(stray.exists());
        assert!(logs.join(file_name(9)).is_symlink());
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn excessive_directory_entries_fail_without_unbounded_enumeration() {
        let root = test_root("bounded");
        let store = store(&root);
        assert_eq!(store.report(&record()), Ok(1));
        let logs = store.fixture_logs_path();
        for index in 0..=super::files::MAX_DIRECTORY_ENTRIES {
            fs::write(logs.join(format!("unrelated-{index}")), "x")
                .expect("unrelated fixture is written");
        }
        assert_eq!(store.report(&record()), Err(CrashReportError::WriteFailed));
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }

    #[test]
    fn a_link_or_permissive_private_directory_fails_closed() {
        let link_root = test_root("directory-link");
        let linked_store = store(&link_root);
        symlink(link_root.join("elsewhere"), link_root.join("Anodrel"))
            .expect("private-tree link fixture is created");
        assert_eq!(
            linked_store.report(&record()),
            Err(CrashReportError::LocationUnavailable)
        );
        fs::remove_dir_all(link_root).expect("linked fixture tree is removable");

        let permissive_root = test_root("directory-permissions");
        let permissive_store = store(&permissive_root);
        let anodrel = permissive_root.join("Anodrel");
        fs::create_dir(&anodrel).expect("private directory fixture is created");
        fs::set_permissions(&anodrel, fs::Permissions::from_mode(0o755))
            .expect("permissive fixture mode is set");
        assert_eq!(
            permissive_store.report(&record()),
            Err(CrashReportError::LocationUnavailable)
        );
        fs::remove_dir_all(permissive_root).expect("permissive fixture tree is removable");
    }

    #[test]
    fn root_shape_sequence_and_debug_stay_closed() {
        assert_eq!(
            home_from_local_data_root(Path::new("/home/anodrel/.local/share")),
            Some(PathBuf::from("/home/anodrel"))
        );
        assert_eq!(
            home_from_local_data_root(Path::new("/home/anodrel/share")),
            None
        );
        assert_eq!(sequence_of(&file_name(42)), Some(42));
        assert_eq!(sequence_of("crash-0.anodrel.v1"), None);
        assert_eq!(sequence_of("crash-042.anodrel.v1"), None);
        assert!(matches!(
            LinuxCrashStore::from_parts(PathBuf::from("relative"), PathBuf::from("logs"), 0),
            Err(LinuxCrashInitializationError::Unavailable)
        ));
        let root = test_root("debug");
        let store = store(&root);
        assert_eq!(format!("{store:?}"), "LinuxCrashStore(..)");
        fs::remove_dir_all(root).expect("fixture tree is removable");
    }
}
