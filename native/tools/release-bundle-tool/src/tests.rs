//! Integration-style checks for the bounded owned bundle authoring boundary.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anodrel_release_bundle::ReleaseBundle;

use crate::{BundleAuthorError, create_release_bundle, source::source_entries_for_test};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn creates_a_canonical_bundle_from_one_normal_source_tree() {
    let directory = TemporaryDirectory::new();
    let source = directory.path().join("source");
    let output = directory.path().join("release.bundle");
    fs::create_dir(&source).expect("source is created");
    fs::create_dir(source.join("content")).expect("content directory is created");
    fs::write(source.join("z-last.txt"), b"last").expect("last source is written");
    fs::write(source.join("content").join("first.txt"), b"first").expect("first source is written");

    create_release_bundle(&source, &output).expect("bundle is created");
    let bundle_bytes = fs::read(&output).expect("bundle is readable");
    let bundle = ReleaseBundle::parse(&bundle_bytes).expect("output reparses");
    let paths = bundle
        .entries()
        .iter()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    assert_eq!(paths, ["content/first.txt", "z-last.txt"]);
    assert_eq!(bundle.file("content/first.txt"), Some(&b"first"[..]));
    assert_eq!(bundle.file("z-last.txt"), Some(&b"last"[..]));
}

#[test]
fn deterministic_collection_sorts_by_raw_utf8_path_bytes() {
    let directory = TemporaryDirectory::new();
    let source = directory.path().join("source");
    let output = directory.path().join("release.bundle");
    fs::create_dir(&source).expect("source is created");
    fs::write(source.join("z.txt"), b"z").expect("z source is written");
    fs::write(source.join("A.txt"), b"a").expect("a source is written");

    let entries = source_entries_for_test(&source, &output).expect("source is collected");
    assert_eq!(
        entries
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>(),
        ["A.txt", "z.txt"]
    );
}

#[test]
fn refuses_to_overwrite_an_existing_output() {
    let directory = TemporaryDirectory::new();
    let source = directory.path().join("source");
    let output = directory.path().join("release.bundle");
    fs::create_dir(&source).expect("source is created");
    fs::write(source.join("content.txt"), b"content").expect("source is written");
    fs::write(&output, b"do not overwrite").expect("existing output is written");

    assert!(matches!(
        create_release_bundle(&source, &output),
        Err(BundleAuthorError::OutputAlreadyExists)
    ));
    assert_eq!(
        fs::read(output).expect("existing output remains"),
        b"do not overwrite"
    );
}

#[test]
fn refuses_an_output_inside_its_source_tree() {
    let directory = TemporaryDirectory::new();
    let source = directory.path().join("source");
    let output = source.join("release.bundle");
    fs::create_dir(&source).expect("source is created");
    fs::write(source.join("content.txt"), b"content").expect("source is written");

    assert!(matches!(
        create_release_bundle(&source, &output),
        Err(BundleAuthorError::OutputInvalid)
    ));
    assert!(!output.exists());
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        for _ in 0..32 {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "anodrel-release-bundle-tool-test-{}-{suffix}",
                std::process::id()
            ));
            if fs::create_dir(&path).is_ok() {
                return Self(path);
            }
        }
        panic!("a temporary bundle authoring directory could not be created");
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
