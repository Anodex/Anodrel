//! Test-only isolated temporary directories allocated per process.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

const REMOVE_ATTEMPTS: u32 = 8;
static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A newly created private test directory below the operating system's temp root.
pub(crate) struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    /// Reserves an absent directory before any fixture writes into it.
    pub(crate) fn new(label: &str) -> Self {
        let sequence = DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("anodrel-{label}-{}-{sequence}", std::process::id(),));
        fs::create_dir(&path).expect("test directory is newly created");
        Self { path }
    }

    /// Returns this fixture's root directory for contained files.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Removes only this fixture directory after bounded Windows sharing retries.
    pub(crate) fn remove(self) {
        for attempt in 1..=REMOVE_ATTEMPTS {
            match fs::remove_dir_all(&self.path) {
                Ok(()) => return,
                Err(error)
                    if error.kind() == std::io::ErrorKind::PermissionDenied
                        && attempt < REMOVE_ATTEMPTS =>
                {
                    thread::sleep(Duration::from_millis(u64::from(attempt) * 10));
                }
                Err(error) => panic!("test directory is removed: {error}"),
            }
        }
        unreachable!("test directory cleanup either succeeds or reports its final error");
    }
}
