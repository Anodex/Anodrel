//! Isolated temporary directories used only by Windows-installer tests.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

const DIRECTORY_ATTEMPTS: u32 = 32;
const REMOVE_ATTEMPTS: u32 = 8;
static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A newly created private test directory below the operating system temp root.
pub(crate) struct TestDirectory {
    path: Option<PathBuf>,
}

impl TestDirectory {
    /// Reserves a fresh directory before a fixture writes its first file.
    pub(crate) fn new(label: &str) -> Self {
        for _ in 0..DIRECTORY_ATTEMPTS {
            let sequence = DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "anodrel-windows-installer-{label}-{}-{sequence}",
                std::process::id(),
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path: Some(path) },
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("test directory is created: {error}"),
            }
        }
        panic!("test directory remains unavailable after {DIRECTORY_ATTEMPTS} attempts");
    }

    /// Returns this fixture's root directory for contained files.
    pub(crate) fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("test directory remains available until cleanup")
    }

    /// Removes this fixture directory and reports a persistent cleanup failure.
    pub(crate) fn remove(mut self) {
        let path = self.path.take().expect("test directory is removed once");
        remove_tree(&path).expect("test directory is removed");
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = remove_tree(&path);
        }
    }
}

fn remove_tree(path: &Path) -> io::Result<()> {
    for attempt in 1..=REMOVE_ATTEMPTS {
        match fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(error)
                if error.kind() == io::ErrorKind::PermissionDenied && attempt < REMOVE_ATTEMPTS =>
            {
                thread::sleep(Duration::from_millis(u64::from(attempt) * 10));
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("test directory cleanup either completes or returns its final error");
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, thread};

    use super::TestDirectory;

    #[test]
    fn concurrent_directories_are_unique_and_private() {
        const DIRECTORY_COUNT: usize = 16;
        let directories = thread::scope(|scope| {
            (0..DIRECTORY_COUNT)
                .map(|_| scope.spawn(|| TestDirectory::new("concurrent")))
                .map(|worker| worker.join().expect("fixture creation completes"))
                .collect::<Vec<_>>()
        });
        let paths = directories
            .iter()
            .map(|directory| directory.path().to_path_buf())
            .collect::<BTreeSet<_>>();

        assert_eq!(paths.len(), DIRECTORY_COUNT);
        for directory in directories {
            assert!(directory.path().is_dir());
            directory.remove();
        }
    }

    #[test]
    fn dropping_a_directory_removes_its_fixture_files() {
        let path = {
            let directory = TestDirectory::new("drop");
            let path = directory.path().to_path_buf();
            std::fs::write(path.join("fixture.txt"), b"fixture data")
                .expect("fixture file is written");
            path
        };

        assert!(!path.exists());
    }
}
