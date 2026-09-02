//! Integration-style checks for owned bundle-derived manifest authoring.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anodrel_application::sha256;
use anodrel_release_bundle::{BundleEntryInput, encode};
use anodrel_windows_installer::{ReleaseManifest, verify_bundle};

use crate::{ReleaseManifestAuthorError, create_release_manifest};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

const PUBLISHER: &str = "a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5";

#[test]
fn derives_identity_and_every_digest_from_the_checked_bundle() {
    let directory = TemporaryDirectory::new();
    let plan = directory.path().join("release-plan.json");
    let bundle = directory.path().join("release.bundle");
    let output = directory.path().join("release-manifest.json");
    let executable = b"Anodrel product executable";
    let bundle_bytes = valid_bundle(executable);
    fs::write(&plan, valid_plan()).expect("plan is written");
    fs::write(&bundle, &bundle_bytes).expect("bundle is written");

    create_release_manifest(&plan, &bundle, &output).expect("manifest is authored");
    let text = fs::read_to_string(&output).expect("manifest output is readable");
    let manifest = ReleaseManifest::parse(&text).expect("output is a strict manifest");
    assert_eq!(
        manifest.application_id(),
        "org.anodrel.release-manifest-test"
    );
    assert_eq!(manifest.executable_path(), "bin/product.exe");
    assert!(manifest.matches_executable_digest(sha256::digest(executable)));
    assert!(manifest.matches_publisher_fingerprint(
        sha256::parse_lower_hex(PUBLISHER).expect("fixture publisher is valid")
    ));
    assert!(verify_bundle(&manifest, &bundle_bytes).is_ok());
}

#[test]
fn version_one_one_plan_derives_a_signed_catalogue_source() {
    let directory = TemporaryDirectory::new();
    let plan = directory.path().join("release-plan.json");
    let bundle = directory.path().join("release.bundle");
    let output = directory.path().join("release-manifest.json");
    let plan_text = valid_plan()
        .replace(
            "\"formatVersion\":{\"major\":1,\"minor\":0}",
            "\"formatVersion\":{\"major\":1,\"minor\":1}",
        )
        .replace(
            "\"networkOrigins\":[]}",
            "\"networkOrigins\":[],\"updateCatalogue\":{\"origin\":{\"host\":\"updates.example.test\",\"port\":443},\"path\":\"/anodrel/stable.p7s\"}}",
        );
    fs::write(&plan, plan_text).expect("version 1.1 plan is written");
    fs::write(&bundle, valid_bundle(b"product")).expect("bundle is written");

    create_release_manifest(&plan, &bundle, &output).expect("manifest is authored");
    let manifest =
        ReleaseManifest::parse(&fs::read_to_string(output).expect("manifest output is readable"))
            .expect("derived manifest is valid");
    assert_eq!(
        manifest
            .update_catalogue_location()
            .expect("signed catalogue source is present")
            .request_path(),
        "/anodrel/stable.p7s"
    );
}

#[test]
fn version_one_two_plan_derives_signed_product_display_metadata() {
    let directory = TemporaryDirectory::new();
    let plan = directory.path().join("release-plan.json");
    let bundle = directory.path().join("release.bundle");
    let output = directory.path().join("release-manifest.json");
    let plan_text = valid_plan()
        .replace(
            "\"formatVersion\":{\"major\":1,\"minor\":0}",
            "\"formatVersion\":{\"major\":1,\"minor\":2}",
        )
        .replace(
            "\"networkOrigins\":[]}",
            "\"networkOrigins\":[],\"updateCatalogue\":{\"origin\":{\"host\":\"updates.example.test\",\"port\":443},\"path\":\"/anodrel/stable.p7s\"},\"product\":{\"displayName\":\"Release Manifest Test\",\"publisherName\":\"Anodrel\"}}",
        );
    fs::write(&plan, plan_text).expect("version 1.2 plan is written");
    fs::write(&bundle, valid_bundle(b"product")).expect("bundle is written");

    create_release_manifest(&plan, &bundle, &output).expect("manifest is authored");
    let manifest =
        ReleaseManifest::parse(&fs::read_to_string(output).expect("manifest output is readable"))
            .expect("derived manifest is valid");
    let product = manifest
        .product_metadata()
        .expect("derived manifest has signed display metadata");
    assert_eq!(product.display_name(), "Release Manifest Test");
    assert_eq!(product.publisher_name(), "Anodrel");
}

#[test]
fn does_not_overwrite_an_existing_manifest_output() {
    let directory = TemporaryDirectory::new();
    let plan = directory.path().join("release-plan.json");
    let bundle = directory.path().join("release.bundle");
    let output = directory.path().join("release-manifest.json");
    fs::write(&plan, valid_plan()).expect("plan is written");
    fs::write(&bundle, valid_bundle(b"product")).expect("bundle is written");
    fs::write(&output, b"do not overwrite").expect("existing output is written");

    assert!(matches!(
        create_release_manifest(&plan, &bundle, &output),
        Err(ReleaseManifestAuthorError::OutputAlreadyExists)
    ));
    assert_eq!(
        fs::read(output).expect("existing output remains"),
        b"do not overwrite"
    );
}

#[test]
fn refuses_a_plan_that_attempts_to_supply_an_application_identity() {
    let directory = TemporaryDirectory::new();
    let plan = directory.path().join("release-plan.json");
    let bundle = directory.path().join("release.bundle");
    let output = directory.path().join("release-manifest.json");
    let invalid = valid_plan().replacen(
        "{\"formatVersion\"",
        "{\"applicationId\":\"org.anodrel.override\",\"formatVersion\"",
        1,
    );
    fs::write(&plan, invalid).expect("invalid plan is written");
    fs::write(&bundle, valid_bundle(b"product")).expect("bundle is written");

    assert!(matches!(
        create_release_manifest(&plan, &bundle, &output),
        Err(ReleaseManifestAuthorError::PlanInvalid)
    ));
    assert!(!output.exists());
}

#[test]
fn refuses_a_bundle_with_application_content_that_does_not_match_its_manifest() {
    let directory = TemporaryDirectory::new();
    let plan = directory.path().join("release-plan.json");
    let bundle = directory.path().join("release.bundle");
    let output = directory.path().join("release-manifest.json");
    let content = b"changed content";
    let application_manifest = application_manifest(b"original content");
    let bytes = encode(&[
        BundleEntryInput {
            path: "anodrel.application.json",
            contents: &application_manifest,
        },
        BundleEntryInput {
            path: "bin/product.exe",
            contents: b"product",
        },
        BundleEntryInput {
            path: "content/main.txt",
            contents: content,
        },
    ])
    .expect("fixture bundle encodes");
    fs::write(&plan, valid_plan()).expect("plan is written");
    fs::write(&bundle, bytes).expect("bundle is written");

    assert!(matches!(
        create_release_manifest(&plan, &bundle, &output),
        Err(ReleaseManifestAuthorError::ApplicationContentInvalid)
    ));
    assert!(!output.exists());
}

fn valid_plan() -> String {
    format!(
        r#"{{"formatVersion":{{"major":1,"minor":0}},"packageVersion":{{"major":1,"minor":2,"patch":3}},"executable":{{"path":"bin/product.exe"}},"publisher":{{"leafCertificateSha256":"{PUBLISHER}"}},"capabilities":[],"networkOrigins":[]}}"#
    )
}

fn valid_bundle(executable: &[u8]) -> Vec<u8> {
    let content = b"Anodrel release-manifest test content\n";
    let application_manifest = application_manifest(content);
    encode(&[
        BundleEntryInput {
            path: "anodrel.application.json",
            contents: &application_manifest,
        },
        BundleEntryInput {
            path: "bin/product.exe",
            contents: executable,
        },
        BundleEntryInput {
            path: "content/main.txt",
            contents: content,
        },
    ])
    .expect("fixture bundle encodes")
}

fn application_manifest(content: &[u8]) -> Vec<u8> {
    let digest = sha256::to_lower_hex(&sha256::digest(content));
    format!(
        r#"{{"manifestVersion":{{"major":1,"minor":0}},"applicationId":"org.anodrel.release-manifest-test","displayName":"Release Manifest Test","content":{{"format":"anodrel.text.v1","path":"content/main.txt","sha256":"{digest}"}}}}"#
    )
    .into_bytes()
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        for _ in 0..32 {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "anodrel-release-manifest-test-{}-{suffix}",
                std::process::id()
            ));
            if fs::create_dir(&path).is_ok() {
                return Self(path);
            }
        }
        panic!("a temporary manifest-authoring directory could not be created");
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
