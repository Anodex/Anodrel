//! Contract tests for the owned installer release-manifest foundation.

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anodrel_application::{InstalledApplication, sha256};
use anodrel_release_bundle::{BundleEntryInput, ReleaseBundleError, encode};

use crate::{
    MAX_PAYLOAD_BYTES, ReleaseManifest, ReleaseManifestError, ReleasePayloadError, verify_bundle,
};

const PUBLISHER: &str = "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585";
const PAYLOAD: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
static NEXT_STAGED_PACKAGE: AtomicU64 = AtomicU64::new(0);

fn release_manifest(executable_digest: &str, capabilities: &str, origins: &str) -> String {
    format!(
        r#"{{
  "formatVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.installer-test",
  "packageVersion": {{ "major": 1, "minor": 2, "patch": 3 }},
  "executable": {{ "path": "bin/Product.EXE", "sha256": "{executable_digest}" }},
  "publisher": {{ "leafCertificateSha256": "{PUBLISHER}" }},
  "capabilities": {capabilities},
  "networkOrigins": {origins},
  "payload": {{ "byteLength": 32, "sha256": "{PAYLOAD}" }}
}}"#
    )
}

fn release_manifest_with_payload(payload: &[u8]) -> String {
    let digest = sha256::to_lower_hex(&sha256::digest(payload));
    release_manifest(PAYLOAD, "[]", "[]").replace(
        &format!("\"byteLength\": 32, \"sha256\": \"{PAYLOAD}\""),
        &format!(
            "\"byteLength\": {}, \"sha256\": \"{digest}\"",
            payload.len()
        ),
    )
}

#[test]
fn a_valid_manifest_preserves_the_signed_release_facts() {
    let release = ReleaseManifest::parse(&release_manifest(PAYLOAD, "[\"session.close\"]", "[]"))
        .expect("the release manifest is valid");

    assert_eq!(release.application_id(), "org.anodrel.installer-test");
    assert_eq!(release.package_version().major(), 1);
    assert_eq!(release.package_version().minor(), 2);
    assert_eq!(release.package_version().patch(), 3);
    assert_eq!(release.executable_path(), "bin/Product.EXE");
    assert!(release.matches_executable_digest(
        sha256::parse_lower_hex(PAYLOAD).expect("digest fixture is valid")
    ));
    assert!(release.matches_publisher_fingerprint(
        sha256::parse_lower_hex(PUBLISHER).expect("publisher fixture is valid")
    ));
    assert_eq!(release.payload().byte_length(), 32);
    assert!(
        release
            .payload()
            .matches_digest(sha256::parse_lower_hex(PAYLOAD).expect("payload fixture is valid"))
    );
}

#[test]
fn the_rendered_machine_record_passes_the_existing_host_validator() {
    let package = StagedPackage::new();
    let executable_digest = package.executable_digest();
    let release = ReleaseManifest::parse(&release_manifest(
        &executable_digest,
        "[\"ui.document.write\", \"session.close\"]",
        "[]",
    ))
    .expect("the release manifest is valid");
    let record = release.render_install_record(package.root());

    let installed =
        InstalledApplication::load_from_trusted_record(&record, "org.anodrel.installer-test")
            .expect("the host accepts the installer-rendered record");
    assert_eq!(
        installed.identity().application_id(),
        release.application_id()
    );
    assert_eq!(installed.capabilities().len(), 2);
}

#[test]
fn network_permission_requires_one_exact_origin_and_canonicalizes_it() {
    let release = ReleaseManifest::parse(&release_manifest(
        PAYLOAD,
        "[\"network.fetch\"]",
        "[{\"host\": \"Api.Example.test\", \"port\": 443}]",
    ))
    .expect("the exact network policy is valid");

    assert_eq!(release.network_origins()[0].hostname(), "api.example.test");
    assert_eq!(release.network_origins()[0].port(), 443);

    let missing_origins =
        ReleaseManifest::parse(&release_manifest(PAYLOAD, "[\"network.fetch\"]", "[]"));
    assert!(matches!(
        missing_origins,
        Err(ReleaseManifestError::PolicyInvalid)
    ));
}

#[test]
fn version_one_one_binds_one_catalogue_source_into_the_machine_record() {
    let package = StagedPackage::new();
    let executable_digest = package.executable_digest();
    let manifest = release_manifest(&executable_digest, "[]", "[]")
        .replace("\"minor\": 0", "\"minor\": 1")
        .replace(
            "  \"payload\":",
            "  \"updateCatalogue\": {\n    \"origin\": { \"host\": \"updates.example.test\", \"port\": 443 },\n    \"path\": \"/anodrel/stable.p7s\"\n  },\n  \"payload\":",
        );
    let release = ReleaseManifest::parse(&manifest).expect("version 1.1 manifest is valid");
    let location = release
        .update_catalogue_location()
        .expect("signed source is present");
    assert_eq!(location.origin().hostname(), "updates.example.test");
    assert_eq!(location.request_path(), "/anodrel/stable.p7s");

    let record = release.render_install_record(package.root());
    let installed =
        InstalledApplication::load_from_trusted_record(&record, "org.anodrel.installer-test")
            .expect("the version 1.20 record is valid");
    assert_eq!(
        installed
            .update_catalogue_location()
            .expect("installed source is present")
            .request_path(),
        "/anodrel/stable.p7s"
    );
}

#[test]
fn version_one_two_binds_safe_product_metadata_to_the_signed_release() {
    let package = StagedPackage::new();
    let manifest = release_manifest(&package.executable_digest(), "[]", "[]")
        .replace("\"minor\": 0", "\"minor\": 2")
        .replace(
            "  \"payload\":",
            "  \"updateCatalogue\": {\n    \"origin\": { \"host\": \"updates.example.test\", \"port\": 443 },\n    \"path\": \"/anodrel/stable.p7s\"\n  },\n  \"product\": {\n    \"displayName\": \"Anodrel Installer Test\",\n    \"publisherName\": \"Anodrel\"\n  },\n  \"payload\":",
        );
    let release = ReleaseManifest::parse(&manifest).expect("version 1.2 manifest is valid");
    let product = release
        .product_metadata()
        .expect("version 1.2 has signed display metadata");
    assert_eq!(product.display_name(), "Anodrel Installer Test");
    assert_eq!(product.publisher_name(), "Anodrel");

    let record = release.render_install_record(package.root());
    let installed =
        InstalledApplication::load_from_trusted_record(&record, "org.anodrel.installer-test")
            .expect("the version 1.21 record is valid");
    let selected_product = installed
        .product_metadata()
        .expect("selected record retains signed display metadata");
    assert_eq!(selected_product.display_name(), "Anodrel Installer Test");
    assert_eq!(selected_product.publisher_name(), "Anodrel");

    let unsafe_text = manifest.replace("Anodrel Installer Test", "Anodrel\u{202E}Test");
    assert!(matches!(
        ReleaseManifest::parse(&unsafe_text),
        Err(ReleaseManifestError::ProductMetadataInvalid)
    ));
}

#[test]
fn version_one_three_binds_a_windows_safe_start_menu_name_to_selected_policy() {
    let package = StagedPackage::new();
    let manifest = release_manifest(&package.executable_digest(), "[]", "[]")
        .replace("\"minor\": 0", "\"minor\": 3")
        .replace(
            "  \"payload\":",
            "  \"updateCatalogue\": {\n    \"origin\": { \"host\": \"updates.example.test\", \"port\": 443 },\n    \"path\": \"/anodrel/stable.p7s\"\n  },\n  \"product\": {\n    \"displayName\": \"Anodrel Installer Test\",\n    \"publisherName\": \"Anodrel\",\n    \"startMenuName\": \"Anodrel Installer Test\"\n  },\n  \"payload\":",
        );
    let release = ReleaseManifest::parse(&manifest).expect("version 1.3 manifest is valid");
    assert_eq!(
        release
            .start_menu_name()
            .expect("version 1.3 has a Start-menu name")
            .as_str(),
        "Anodrel Installer Test"
    );

    let record = release.render_install_record(package.root());
    let installed =
        InstalledApplication::load_from_trusted_record(&record, "org.anodrel.installer-test")
            .expect("the version 1.22 record is valid");
    assert_eq!(
        installed
            .start_menu_name()
            .expect("selected record retains the Start-menu name")
            .as_str(),
        "Anodrel Installer Test"
    );

    let unsafe_name = manifest.replace("Anodrel Installer Test\"\n  },", "NUL\"\n  },");
    assert!(matches!(
        ReleaseManifest::parse(&unsafe_name),
        Err(ReleaseManifestError::ProductMetadataInvalid)
    ));
}

#[test]
fn version_one_four_binds_a_distinct_product_launcher_to_selected_policy() {
    let package = StagedPackage::new();
    let launcher_digest = package.launcher_digest();
    let manifest = release_manifest(&package.executable_digest(), "[]", "[]")
        .replace("\"minor\": 0", "\"minor\": 4")
        .replace(
            "  \"payload\":",
            &format!(
                "  \"updateCatalogue\": {{\n    \"origin\": {{ \"host\": \"updates.example.test\", \"port\": 443 }},\n    \"path\": \"/anodrel/stable.p7s\"\n  }},\n  \"product\": {{\n    \"displayName\": \"Anodrel Installer Test\",\n    \"publisherName\": \"Anodrel\",\n    \"startMenuName\": \"Anodrel Installer Test\"\n  }},\n  \"launcher\": {{ \"path\": \"bin/anodrel-windows-host.exe\", \"sha256\": \"{launcher_digest}\" }},\n  \"payload\":"
            ),
        );
    let release = ReleaseManifest::parse(&manifest).expect("version 1.4 manifest is valid");
    assert_eq!(
        release
            .product_launcher()
            .expect("version 1.4 has a launcher")
            .path(),
        "bin/anodrel-windows-host.exe"
    );

    let record = release.render_install_record(package.root());
    let installed =
        InstalledApplication::load_from_trusted_record(&record, "org.anodrel.installer-test")
            .expect("the version 1.23 record is valid");
    assert_eq!(
        installed
            .product_launcher_path()
            .expect("selected record retains the launcher"),
        std::fs::canonicalize(package.launcher_path())
            .expect("the staged launcher canonicalizes")
            .as_path()
    );
    let mut launcher = std::fs::File::open(package.launcher_path()).expect("launcher opens");
    installed
        .revalidate_product_launcher(&package.launcher_path(), &mut launcher)
        .expect("launcher revalidates through its record digest");

    let same_as_child = manifest.replace("bin/anodrel-windows-host.exe", "bin/Product.EXE");
    assert!(matches!(
        ReleaseManifest::parse(&same_as_child),
        Err(ReleaseManifestError::ExecutablePathInvalid)
    ));
}

#[test]
fn malformed_paths_unknown_fields_and_out_of_bounds_payloads_fail_closed() {
    let parent_path =
        release_manifest(PAYLOAD, "[]", "[]").replace("bin/Product.EXE", "bin/../Product.EXE");
    assert!(matches!(
        ReleaseManifest::parse(&parent_path),
        Err(ReleaseManifestError::ExecutablePathInvalid)
    ));

    let unknown_field = release_manifest(PAYLOAD, "[]", "[]")
        .replace("  \"payload\":", "  \"unexpected\": true,\n  \"payload\":");
    assert!(matches!(
        ReleaseManifest::parse(&unknown_field),
        Err(ReleaseManifestError::Invalid)
    ));

    let excessive_payload = release_manifest(PAYLOAD, "[]", "[]").replace(
        "\"byteLength\": 32",
        &format!("\"byteLength\": {}", MAX_PAYLOAD_BYTES + 1),
    );
    assert!(matches!(
        ReleaseManifest::parse(&excessive_payload),
        Err(ReleaseManifestError::PayloadInvalid)
    ));
}

#[test]
fn the_signed_payload_must_match_its_manifest_before_the_bundle_is_usable() {
    let payload = encode(&[BundleEntryInput {
        path: "content/main.txt",
        contents: b"verified release content",
    }])
    .expect("the owned bundle encodes");
    let manifest = ReleaseManifest::parse(&release_manifest_with_payload(&payload))
        .expect("the manifest authenticates the exact payload");
    let bundle = verify_bundle(&manifest, &payload).expect("both release checks pass");
    assert_eq!(
        bundle.file("content/main.txt"),
        Some(&b"verified release content"[..])
    );

    let mut substituted = payload.clone();
    *substituted.last_mut().expect("the payload has bytes") ^= 1;
    assert!(matches!(
        verify_bundle(&manifest, &substituted),
        Err(ReleasePayloadError::DigestMismatch)
    ));

    let malformed = b"not a release bundle";
    let malformed_manifest = ReleaseManifest::parse(&release_manifest_with_payload(malformed))
        .expect("the manifest can authenticate invalid bundle bytes");
    assert!(matches!(
        verify_bundle(&malformed_manifest, malformed),
        Err(ReleasePayloadError::BundleInvalid(
            ReleaseBundleError::HeaderInvalid
        ))
    ));
}

/// A temporary package whose identity and content meet the normal host checks.
struct StagedPackage(PathBuf);

impl StagedPackage {
    fn new() -> Self {
        let sequence = NEXT_STAGED_PACKAGE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "anodrel-windows-installer-test-{}-{sequence}",
            std::process::id(),
        ));
        std::fs::create_dir(&root).expect("temporary package root is created");
        std::fs::create_dir_all(root.join("content")).expect("the content directory is created");
        std::fs::create_dir_all(root.join("bin")).expect("the binary directory is created");
        let content = b"installer record validation";
        std::fs::write(root.join("content").join("main.txt"), content)
            .expect("the package content is written");
        std::fs::write(root.join("bin").join("Product.EXE"), b"placeholder image")
            .expect("the package executable is written");
        std::fs::write(
            root.join("bin").join("anodrel-windows-host.exe"),
            b"placeholder host image",
        )
        .expect("the package launcher is written");
        let content_digest = sha256::to_lower_hex(&sha256::digest(content));
        std::fs::write(
            root.join("anodrel.application.json"),
            format!(
                r#"{{
  "manifestVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.installer-test",
  "displayName": "Installer Test",
  "content": {{
    "format": "anodrel.text.v1",
    "path": "content/main.txt",
    "sha256": "{content_digest}"
  }}
}}
"#
            ),
        )
        .expect("the package manifest is written");
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }

    fn executable_digest(&self) -> String {
        let bytes = std::fs::read(self.0.join("bin").join("Product.EXE"))
            .expect("the package executable is read");
        sha256::to_lower_hex(&sha256::digest(&bytes))
    }

    fn launcher_path(&self) -> PathBuf {
        self.0.join("bin").join("anodrel-windows-host.exe")
    }

    fn launcher_digest(&self) -> String {
        let bytes = std::fs::read(self.launcher_path()).expect("the package launcher is read");
        sha256::to_lower_hex(&sha256::digest(&bytes))
    }
}

impl Drop for StagedPackage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
