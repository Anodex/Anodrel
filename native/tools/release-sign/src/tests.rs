//! Integration-style checks for owned direct Windows release signing.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anodrel_release_bundle::{BundleEntryInput, encode};
use anodrel_release_image::embed_release_image;

use crate::{ReleaseSignError, sign_release_image};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn a_missing_exact_certificate_leaves_no_fresh_copy() {
    let directory = TemporaryDirectory::new();
    let input = directory.path().join("unsigned-release.exe");
    let output = directory.path().join("signed-release.exe");
    let template = std::env::current_exe().expect("the test executable path is available");
    let payload = encode(&[BundleEntryInput {
        path: "content/main.txt",
        contents: b"release payload",
    }])
    .expect("the owned bundle encodes");
    let manifest = manifest_for(&payload, &"0".repeat(64));
    embed_release_image(&template, &input, &manifest, &payload)
        .expect("a checked unsigned image is assembled");

    assert!(matches!(
        sign_release_image(&input, &"0".repeat(64), &output),
        Err(ReleaseSignError::CertificateUnavailable)
    ));
    assert!(input.is_file());
    assert!(!output.exists());
}

#[test]
fn invalid_certificate_text_cannot_create_an_output() {
    let directory = TemporaryDirectory::new();
    let input = directory.path().join("not-an-image.exe");
    let output = directory.path().join("signed-release.exe");
    fs::write(&input, b"not a release image").expect("invalid input is written");

    assert!(matches!(
        sign_release_image(&input, "not-a-fingerprint", &output),
        Err(ReleaseSignError::CertificateFingerprintInvalid)
    ));
    assert!(!output.exists());
}

fn manifest_for(payload: &[u8], publisher: &str) -> Vec<u8> {
    let digest =
        anodrel_application::sha256::to_lower_hex(&anodrel_application::sha256::digest(payload));
    format!(
        r#"{{
  "formatVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.release-sign-test",
  "packageVersion": {{ "major": 1, "minor": 0, "patch": 0 }},
  "executable": {{
    "path": "bin/product.exe",
    "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
  }},
  "publisher": {{
    "leafCertificateSha256": "{publisher}"
  }},
  "capabilities": [],
  "networkOrigins": [],
  "payload": {{ "byteLength": {}, "sha256": "{digest}" }}
}}"#,
        payload.len()
    )
    .into_bytes()
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        for _ in 0..32 {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "anodrel-release-sign-test-{}-{suffix}",
                std::process::id()
            ));
            if fs::create_dir(&path).is_ok() {
                return Self(path);
            }
        }
        panic!("a temporary release-signing directory could not be created");
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
