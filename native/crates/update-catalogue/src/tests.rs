//! Contract checks for strict future update catalogues.

use anodrel_application::sha256;
use anodrel_windows_installer::PackageVersion;

use crate::{MAX_UPDATE_CATALOGUE_BYTES, UpdateCatalogue, UpdateCatalogueError};

const PUBLISHER: &str = "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585";

#[test]
fn valid_catalogue_binds_an_exact_newer_installer_image() {
    let image = b"signed release image bytes";
    let catalogue = UpdateCatalogue::parse(&catalogue(image)).expect("catalogue is valid");

    assert_eq!(catalogue.application_id(), "org.anodrel.update-test");
    assert_eq!(catalogue.package_version(), PackageVersion::new(1, 2, 3));
    assert!(catalogue.matches_installed(
        "org.anodrel.update-test",
        sha256::parse_lower_hex(PUBLISHER).expect("fixture publisher is valid")
    ));
    assert!(catalogue.is_newer_than(PackageVersion::new(1, 2, 2)));
    assert!(!catalogue.is_newer_than(PackageVersion::new(1, 2, 3)));
    assert_eq!(
        catalogue.installer().origin().hostname(),
        "updates.example.test"
    );
    assert_eq!(catalogue.installer().origin().port(), 443);
    assert_eq!(
        catalogue.installer().request_path(),
        "/anodrel/1.2.3/installer.exe"
    );
    assert_eq!(catalogue.installer().byte_length(), image.len() as u64);
    assert!(catalogue.installer().matches_bytes(image));
    assert!(!catalogue.installer().matches_bytes(b"substituted image"));
}

#[test]
fn identity_publisher_and_version_comparisons_are_exact() {
    let catalogue = UpdateCatalogue::parse(&catalogue(b"image")).expect("catalogue is valid");
    assert!(!catalogue.matches_installed(
        "org.anodrel.other",
        sha256::parse_lower_hex(PUBLISHER).expect("fixture publisher is valid")
    ));
    assert!(
        !catalogue.matches_installed(
            "org.anodrel.update-test",
            sha256::parse_lower_hex(
                "9089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"
            )
            .expect("fixture publisher is valid")
        )
    );
    assert!(!catalogue.is_newer_than(PackageVersion::new(9, 0, 0)));
}

#[test]
fn malformed_locations_fields_and_image_descriptors_fail_closed() {
    let image = b"image";
    for invalid in [
        catalogue(image).replace(
            "/anodrel/1.2.3/installer.exe",
            "https://wrong.example/installer.exe",
        ),
        catalogue(image).replace("/anodrel/1.2.3/installer.exe", "/anodrel//installer.exe"),
        catalogue(image).replace("/anodrel/1.2.3/installer.exe", "/anodrel/../installer.exe"),
        catalogue(image).replace("\"byteLength\": 5", "\"byteLength\": 0"),
        catalogue(image).replace("\"sha256\":", "\"unexpected\": true, \"sha256\":"),
    ] {
        assert!(UpdateCatalogue::parse(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn size_and_version_are_bounded_before_a_catalogue_can_be_used() {
    assert!(matches!(
        UpdateCatalogue::parse(&" ".repeat(MAX_UPDATE_CATALOGUE_BYTES + 1)),
        Err(UpdateCatalogueError::TooLarge)
    ));
    assert!(matches!(
        UpdateCatalogue::parse(&catalogue(b"image").replace("\"minor\": 0", "\"minor\": 1")),
        Err(UpdateCatalogueError::VersionUnsupported)
    ));
}

fn catalogue(image: &[u8]) -> String {
    let digest = sha256::to_lower_hex(&sha256::digest(image));
    format!(
        r#"{{
  "formatVersion": {{ "major": 1, "minor": 0 }},
  "applicationId": "org.anodrel.update-test",
  "packageVersion": {{ "major": 1, "minor": 2, "patch": 3 }},
  "publisher": {{ "leafCertificateSha256": "{PUBLISHER}" }},
  "installer": {{
    "origin": {{ "host": "updates.example.test", "port": 443 }},
    "path": "/anodrel/1.2.3/installer.exe",
    "byteLength": {},
    "sha256": "{digest}"
  }}
}}"#,
        image.len()
    )
}
