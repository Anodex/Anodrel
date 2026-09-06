//! Isolated temporary directories used only by Windows-host tests.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

const REMOVE_ATTEMPTS: u32 = 8;
static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A newly created private test directory below the operating system temp root.
pub(super) struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    /// Reserves an absent directory before any fixture writes into it.
    pub(super) fn new(label: &str) -> Self {
        let sequence = DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "anodrel-windows-host-{label}-{}-{sequence}",
            std::process::id(),
        ));
        fs::create_dir(&path).expect("test directory is newly created");
        Self { path }
    }

    /// Returns this fixture's root directory for contained files.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Removes only this fixture directory after bounded Windows sharing retries.
    pub(super) fn remove(self) {
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

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, thread};

    use super::TestDirectory;

    #[test]
    fn concurrent_directories_are_unique_and_private() {
        const FIXTURE_COUNT: usize = 16;
        let directories = thread::scope(|scope| {
            (0..FIXTURE_COUNT)
                .map(|_| scope.spawn(|| TestDirectory::new("concurrent")))
                .map(|worker| worker.join().expect("fixture creation completes"))
                .collect::<Vec<_>>()
        });
        let paths = directories
            .iter()
            .map(|directory| directory.path().to_path_buf())
            .collect::<BTreeSet<_>>();

        assert_eq!(paths.len(), FIXTURE_COUNT);
        for directory in directories {
            assert!(directory.path().is_dir());
            directory.remove();
        }
    }
}
