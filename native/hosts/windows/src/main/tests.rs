//! Unit checks for top-level Windows-host helpers.

#[path = "tests/fixtures.rs"]
mod fixtures;

use anodrel_application::ApplicationPackage;
use anodrel_ui_document::MAX_ENCODED_DOCUMENT_BYTES;

use self::fixtures::TestDirectory;
use crate::startup::{
    check_core_health, health_display, load_ui_preview_document, package_facts,
    read_bounded_regular_utf8,
};

#[test]
fn displays_a_valid_health_response() {
    let display = health_display(
        r#"{"status":"success","result":{"hostName":"test-host","protocolVersion":{"major":1,"minor":0}}}"#,
    )
    .expect("response is valid");
    assert!(display.contains("status: success"));
    assert!(display.contains("protocol: 1.0"));
}

#[test]
fn startup_lab_requires_a_successful_core_check() {
    let display = check_core_health().expect("core health check is valid");
    assert!(display.contains("status: success"));
}

#[test]
fn displays_only_host_verified_application_metadata() {
    let manifest = r#"{
        "manifestVersion":{"major":1,"minor":0},
        "applicationId":"org.anodrel.sample",
        "displayName":"Anodrel Sample",
        "content":{"format":"anodrel.text.v1","path":"content/main.txt","sha256":"7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"}
    }"#;
    let root = TestDirectory::new("host-display");
    let content = root.path().join("content").join("main.txt");
    std::fs::create_dir_all(content.parent().expect("content has parent"))
        .expect("fixture directory is created");
    std::fs::write(root.path().join("anodrel.application.json"), manifest)
        .expect("fixture manifest is written");
    std::fs::write(&content, "Verified package text.").expect("fixture content is written");

    let package = ApplicationPackage::load(root.path().join("anodrel.application.json"))
        .expect("fixture package is valid");
    let facts = package_facts(&package);

    assert_eq!(facts.application_id, "org.anodrel.sample");
    assert_eq!(facts.display_name, "Anodrel Sample");
    assert_eq!(facts.content_format, "anodrel.text.v1");
    assert_eq!(facts.content_path, "content/main.txt");
    assert_eq!(facts.content_bytes, "Verified package text.".len());
    // The facts handed to the window layer carry the verified digest and
    // the declared relative path, never a resolved filesystem location.
    assert_eq!(
        facts.content_digest,
        "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"
    );
    assert!(!facts.content_path.contains(':'));
    root.remove();
}

#[test]
fn preview_loader_reads_one_bounded_valid_document_before_window_creation() {
    let root = TestDirectory::new("ui-preview");
    let document_path = root.path().join("preview.json");
    std::fs::write(
        &document_path,
        r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Preview","fontSize":16,"tone":"primary"}}"#,
    )
    .expect("fixture document is written");

    let document = load_ui_preview_document(&document_path).expect("preview is valid");
    assert_eq!(document.root().id().as_str(), "root");

    let oversized_path = root.path().join("oversized.json");
    std::fs::write(&oversized_path, "x".repeat(MAX_ENCODED_DOCUMENT_BYTES + 1))
        .expect("oversized fixture is written");
    let error = read_bounded_regular_utf8(&oversized_path)
        .expect_err("oversized preview is rejected before decoding");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);

    root.remove();
}

#[test]
fn shipped_ui_preview_document_matches_the_strict_format() {
    let document_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../apps/sample/anodrel.ui.json");
    let document =
        load_ui_preview_document(&document_path).expect("shipped UI preview document is valid");

    assert_eq!(document.root().id().as_str(), "sample.root");
}
