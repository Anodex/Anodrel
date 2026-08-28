//! Shared disposable-directory support for generated-project integration tests.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

const PREFIX: &str = "anodrel-native-app-tool-test";

static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

pub struct TestDirectory {
    pub path: PathBuf,
    parent: PathBuf,
}

impl TestDirectory {
    pub fn new() -> Self {
        let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let parent = repository_root()
            .join("target")
            .join("native-app-tool-tests");
        fs::create_dir_all(&parent).expect("create generated-project test directory");
        let path = parent.join(format!("{PREFIX}-{}-{sequence}", std::process::id()));
        fs::create_dir(&path).expect("create unique generated-project test directory");
        Self { path, parent }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let expected_parent = repository_root()
            .join("target")
            .join("native-app-tool-tests");
        let name = self.path.file_name().and_then(|name| name.to_str());
        if self.parent == expected_parent
            && self.path.parent() == Some(self.parent.as_path())
            && name.is_some_and(|name| name.starts_with(PREFIX))
        {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("native-app-tool lives below the Anodrel repository root")
        .to_path_buf()
}
