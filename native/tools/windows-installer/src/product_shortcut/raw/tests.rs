//! Direct Windows Shell Link checks using a temporary ordinary directory.

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use super::{remove_regular_link, replace_link};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn writes_one_shell_link_in_a_regular_temporary_directory() {
    let directory = TemporaryDirectory::new();
    let executable = std::env::current_exe().expect("current test image is available");
    let link = directory.path().join("Anodrel Test.lnk");
    replace_link(
        &executable,
        executable.parent().expect("test image has a parent"),
        &link,
    )
    .expect("direct Shell Link persistence succeeds");
    assert!(link.is_file());
}

#[test]
fn removes_only_the_regular_shell_link_it_just_created() {
    let directory = TemporaryDirectory::new();
    let executable = std::env::current_exe().expect("current test image is available");
    let link = directory.path().join("Anodrel Test.lnk");
    replace_link(
        &executable,
        executable.parent().expect("test image has a parent"),
        &link,
    )
    .expect("direct Shell Link persistence succeeds");
    remove_regular_link(&link).expect("regular temporary Shell Link removes");
    assert!(!link.exists());
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        for _ in 0..32 {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "anodrel-shortcut-test-{}-{suffix}",
                std::process::id()
            ));
            if std::fs::create_dir(&path).is_ok() {
                return Self(path);
            }
        }
        panic!("temporary Shell Link test directory is unavailable");
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
