//! Integration-style tests for owned resource-image assembly.

use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use anodrel_release_bundle::{BundleEntryInput, encode};
use anodrel_windows_installer::ReleaseManifest;

use crate::{embed_release_image, verify_release_image, verify_release_image_for_publisher};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn manifest_for(payload: &[u8]) -> Vec<u8> {
    let digest =
        anodrel_application::sha256::to_lower_hex(&anodrel_application::sha256::digest(payload));
    format!(
        r#"{{
  "formatVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.release-image-test",
  "packageVersion": {{ "major": 1, "minor": 0, "patch": 0 }},
  "executable": {{
    "path": "bin/product.exe",
    "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
  }},
  "publisher": {{
    "leafCertificateSha256": "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"
  }},
  "capabilities": [],
  "networkOrigins": [],
  "payload": {{ "byteLength": {}, "sha256": "{digest}" }}
}}"#,
        payload.len()
    )
    .into_bytes()
}

#[test]
fn a_new_pe_copy_carries_exact_checked_release_resources() {
    let directory = TemporaryDirectory::new();
    let template = std::env::current_exe().expect("the test executable path is available");
    let output = directory.path.join("release-image.exe");
    let payload = encode(&[BundleEntryInput {
        path: "content/main.txt",
        contents: b"release payload",
    }])
    .expect("the owned bundle encodes");
    let manifest = manifest_for(&payload);
    assert!(
        ReleaseManifest::parse(std::str::from_utf8(&manifest).expect("manifest is UTF-8")).is_ok()
    );

    embed_release_image(&template, &output, &manifest, &payload)
        .expect("the new PE copy receives verified resources");
    assert!(output.is_file());
    verify_release_image(&output).expect("the assembled image reparses as one release");
    assert!(matches!(
        verify_release_image_for_publisher(&output, [0; 32]),
        Err(crate::ReleaseImageError::PublisherMismatch)
    ));
}

#[test]
fn inspection_rejects_an_image_without_the_fixed_release_resources() {
    let image = std::env::current_exe().expect("the test executable path is available");
    assert!(matches!(
        verify_release_image(&image),
        Err(crate::ReleaseImageError::ResourceVerificationFailed)
    ));
}

#[test]
fn assembly_refuses_to_overwrite_an_existing_output() {
    let directory = TemporaryDirectory::new();
    let template = std::env::current_exe().expect("the test executable path is available");
    let output = directory.path.join("existing.exe");
    std::fs::write(&output, b"do not overwrite").expect("the existing output is written");
    let payload = encode(&[BundleEntryInput {
        path: "content/main.txt",
        contents: b"release payload",
    }])
    .expect("the owned bundle encodes");
    let manifest = manifest_for(&payload);

    assert!(matches!(
        embed_release_image(&template, &output, &manifest, &payload),
        Err(crate::ReleaseImageError::OutputAlreadyExists)
    ));
    assert_eq!(
        std::fs::read(output).expect("existing output remains"),
        b"do not overwrite"
    );
}

struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "anodrel-release-image-test-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the temporary directory is created");
        Self { path }
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
