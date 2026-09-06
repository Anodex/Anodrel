//! Contract tests for private staged release extraction.

use std::path::Path;

use anodrel_application::sha256;
use anodrel_release_bundle::{BundleEntryInput, encode};

use crate::staging::stage_checked_release;
use crate::test_support::TestDirectory;
use crate::{ReleaseManifest, StagedReleaseError, verify_bundle};

const PUBLISHER: &str = "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585";

#[test]
fn an_exact_checked_bundle_becomes_a_valid_private_staged_package() {
    let parent = TestDirectory::new("staging");
    let (manifest, payload) = valid_release();
    let manifest = ReleaseManifest::parse(&manifest).expect("the fixture manifest is valid");
    let bundle = verify_bundle(&manifest, &payload).expect("the fixture bundle is valid");

    let staged = stage_checked_release(parent.path(), &manifest, &bundle)
        .expect("the checked bundle creates a staged package");
    let canonical_parent =
        std::fs::canonicalize(parent.path()).expect("the temporary staging parent canonicalizes");
    assert!(staged.package_root.starts_with(canonical_parent));
    assert_eq!(
        std::fs::read(staged.package_root.join("content/main.txt"))
            .expect("the staged content is readable"),
        b"staged release content"
    );
    assert!(staged.install_record.contains("\"recordVersion\""));
    let staged_root = staged.package_root.clone();
    drop(staged);
    assert!(
        !staged_root.exists(),
        "dropping an unpublished stage cleans it up"
    );
}

#[test]
fn a_stage_failure_leaves_no_partial_staging_directory() {
    let parent = TestDirectory::new("staging");
    let (manifest, payload) = valid_release();
    let manifest = ReleaseManifest::parse(&manifest.replace("bin/Product.exe", "bin/Missing.exe"))
        .expect("the altered manifest remains syntactically valid");
    let bundle =
        verify_bundle(&manifest, &payload).expect("the payload still meets its descriptor");

    assert!(matches!(
        stage_checked_release(parent.path(), &manifest, &bundle),
        Err(StagedReleaseError::BundlePathInvalid)
    ));
    assert!(directory_is_empty(parent.path()));
}

#[test]
fn windows_reserved_names_fail_before_files_are_created() {
    let parent = TestDirectory::new("staging");
    let executable = b"staged executable";
    let package = package_manifest();
    let payload = encode(&[
        BundleEntryInput {
            path: "anodrel.application.json",
            contents: &package,
        },
        BundleEntryInput {
            path: "bin/Product.exe",
            contents: executable,
        },
        BundleEntryInput {
            path: "content/CON.txt",
            contents: b"unsafe name",
        },
        BundleEntryInput {
            path: "content/main.txt",
            contents: b"staged release content",
        },
    ])
    .expect("the portable bundle permits the Windows-specific edge");
    let manifest = ReleaseManifest::parse(&release_manifest(&payload, executable))
        .expect("the release manifest is valid");
    let bundle = verify_bundle(&manifest, &payload).expect("the release bundle is valid");

    assert!(matches!(
        stage_checked_release(parent.path(), &manifest, &bundle),
        Err(StagedReleaseError::BundlePathInvalid)
    ));
    assert!(directory_is_empty(parent.path()));
}

fn valid_release() -> (String, Vec<u8>) {
    let executable = b"staged executable";
    let package = package_manifest();
    let payload = encode(&[
        BundleEntryInput {
            path: "anodrel.application.json",
            contents: &package,
        },
        BundleEntryInput {
            path: "bin/Product.exe",
            contents: executable,
        },
        BundleEntryInput {
            path: "content/main.txt",
            contents: b"staged release content",
        },
    ])
    .expect("the fixture bundle encodes");
    (release_manifest(&payload, executable), payload)
}

fn package_manifest() -> Vec<u8> {
    let digest = sha256::to_lower_hex(&sha256::digest(b"staged release content"));
    format!(
        r#"{{
  "manifestVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.staging-test",
  "displayName": "Staging Test",
  "content": {{
    "format": "anodrel.text.v1",
    "path": "content/main.txt",
    "sha256": "{digest}"
  }}
}}"#
    )
    .into_bytes()
}

fn release_manifest(payload: &[u8], executable: &[u8]) -> String {
    let payload_digest = sha256::to_lower_hex(&sha256::digest(payload));
    let executable_digest = sha256::to_lower_hex(&sha256::digest(executable));
    format!(
        r#"{{
  "formatVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.staging-test",
  "packageVersion": {{ "major": 1, "minor": 2, "patch": 3 }},
  "executable": {{ "path": "bin/Product.exe", "sha256": "{executable_digest}" }},
  "publisher": {{ "leafCertificateSha256": "{PUBLISHER}" }},
  "capabilities": ["ui.document.write", "session.close"],
  "networkOrigins": [],
  "payload": {{ "byteLength": {}, "sha256": "{payload_digest}" }}
}}"#,
        payload.len()
    )
}

fn directory_is_empty(path: &Path) -> bool {
    std::fs::read_dir(path)
        .expect("the staging parent is readable")
        .next()
        .is_none()
}
