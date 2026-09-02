//! Integration-style checks for fresh signed update-catalogue authoring.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anodrel_windows_signing::WindowsSigningError;
use anodrel_windows_update_catalogue_signature::UpdateCatalogueSignatureError;

use crate::{UpdateCatalogueSignToolError, sign_catalogue_file};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn a_missing_exact_certificate_leaves_no_fresh_catalogue_output() {
    let directory = TemporaryDirectory::new();
    let input = directory.path().join("catalogue.json");
    let output = directory.path().join("catalogue.p7s");
    fs::write(&input, catalogue()).expect("catalogue is written");

    assert!(matches!(
        sign_catalogue_file(&input, &"0".repeat(64), &output),
        Err(UpdateCatalogueSignToolError::SignatureInvalid(
            UpdateCatalogueSignatureError::SignatureInvalid(
                WindowsSigningError::CertificateUnavailable
            )
        ))
    ));
    assert!(!output.exists());
}

#[test]
fn an_existing_output_is_never_replaced_before_certificate_selection() {
    let directory = TemporaryDirectory::new();
    let input = directory.path().join("catalogue.json");
    let output = directory.path().join("catalogue.p7s");
    fs::write(&input, catalogue()).expect("catalogue is written");
    fs::write(&output, b"do not replace").expect("existing output is written");

    assert!(matches!(
        sign_catalogue_file(&input, &"0".repeat(64), &output),
        Err(UpdateCatalogueSignToolError::OutputAlreadyExists)
    ));
    assert_eq!(
        fs::read(output).expect("existing output remains"),
        b"do not replace"
    );
}

#[test]
fn an_invalid_fingerprint_cannot_create_an_output() {
    let directory = TemporaryDirectory::new();
    let input = directory.path().join("catalogue.json");
    let output = directory.path().join("catalogue.p7s");
    fs::write(&input, catalogue()).expect("catalogue is written");

    assert!(matches!(
        sign_catalogue_file(&input, "not-a-fingerprint", &output),
        Err(UpdateCatalogueSignToolError::CertificateFingerprintInvalid)
    ));
    assert!(!output.exists());
}

fn catalogue() -> String {
    r#"{
  "formatVersion": { "major": 1, "minor": 0 },
  "applicationId": "org.anodrel.catalogue-sign-test",
  "packageVersion": { "major": 1, "minor": 2, "patch": 3 },
  "publisher": {
    "leafCertificateSha256": "0000000000000000000000000000000000000000000000000000000000000000"
  },
  "installer": {
    "origin": { "host": "updates.example.test", "port": 443 },
    "path": "/releases/1.2.3/anodrel-installer.exe",
    "byteLength": 1,
    "sha256": "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
  }
}"#
    .to_owned()
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        for _ in 0..32 {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "anodrel-update-catalogue-sign-test-{}-{suffix}",
                std::process::id()
            ));
            if fs::create_dir(&path).is_ok() {
                return Self(path);
            }
        }
        panic!("a temporary update-catalogue-signing directory could not be created");
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
