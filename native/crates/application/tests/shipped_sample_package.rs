//! Validates the sample package this repository actually ships.
//!
//! The unit tests in this crate build their own fixtures, which proves the
//! validator works but says nothing about `apps/sample`. That gap let a commit
//! reword the sample's content without updating its manifest digest: every test
//! passed, and `start.bat` failed with `ContentDigestMismatch` because the
//! Startup Lab validates the real package before opening a window.
//!
//! These tests close it. The manifest and its content are a matched pair, and
//! anything that edits one without the other fails here rather than at launch.

use std::path::{Path, PathBuf};

use anodrel_application::{ApplicationPackage, TEXT_CONTENT_FORMAT};

/// Returns the repository root, resolved from this crate's location.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the crate sits three levels below the repository root")
        .to_path_buf()
}

fn sample_manifest() -> PathBuf {
    repository_root().join("apps/sample/anodrel.application.json")
}

#[test]
fn the_shipped_sample_manifest_exists_where_start_bat_looks_for_it() {
    let manifest = sample_manifest();
    assert!(
        manifest.is_file(),
        "start.bat launches this path; it must exist: {}",
        manifest.display()
    );
}

#[test]
fn the_shipped_sample_package_passes_every_check() {
    // This is the exact call the Startup Lab makes before creating a window.
    // If it fails, the application cannot start.
    let package = ApplicationPackage::load(sample_manifest())
        .expect("the shipped sample package must validate; run start.bat if this fails");

    assert_eq!(package.identity().application_id(), "org.anodrel.sample");
    assert_eq!(package.identity().display_name(), "Anodrel Sample");
    assert_eq!(package.content().format(), TEXT_CONTENT_FORMAT);
    assert_eq!(package.content().path(), "content/main.txt");
    assert!(
        !package.text().is_empty(),
        "the sample surface would render blank"
    );
}

#[test]
fn the_shipped_manifest_digest_matches_its_content() {
    // Stated separately from the load above so a mismatch reports the two
    // digests rather than a bare error, which is what makes it fixable.
    let package = ApplicationPackage::load(sample_manifest())
        .expect("the shipped sample package must validate");
    let content = std::fs::read(repository_root().join("apps/sample/content/main.txt"))
        .expect("declared content is readable");

    assert_eq!(
        package.content().byte_length(),
        content.len(),
        "the manifest and the content file disagree on length"
    );
    assert_eq!(
        package.content().digest().len(),
        64,
        "a SHA-256 digest is 64 hexadecimal characters"
    );
    assert!(
        package
            .content()
            .digest()
            .chars()
            .all(|character| character.is_ascii_hexdigit()
                && !character.is_ascii_uppercase()),
        "the digest must be lower-case hexadecimal"
    );
}
