#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! First-party strict update catalogue authoring from one locked signed image.
//!
//! This release-operator tool derives update facts from a Windows-verified
//! installer rather than trusting a manifest or digest sidecar. It does not
//! sign, publish, install, launch, or retrieve an update. See
//! `docs/UPDATE_CATALOGUE.md` and Decision 0175.

mod error;
mod output;

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use anodrel_application::sha256;
use anodrel_json::JsonValue;
use anodrel_update_catalogue::{MAX_UPDATE_IMAGE_BYTES, UpdateCatalogue};
use anodrel_windows_installer::{PackageVersion, verify_locked_installer_image};
use anodrel_windows_signature::verify_embedded_signature;

pub use error::UpdateCatalogueCreateError;

/// Derives one fresh strict unsigned catalogue from an accepted signed image.
///
/// The image and output paths must be absolute. `host`, `port`, and
/// `installer_path` name the explicit HTTPS publication location; all release
/// facts come only from the locked image. The result is unsigned JSON and must
/// pass through the separate attached-CMS signing boundary before publication.
pub fn create_update_catalogue(
    image_path: &Path,
    host: &str,
    port: u16,
    installer_path: &str,
    output_path: &Path,
) -> Result<(), UpdateCatalogueCreateError> {
    let image_path = canonical_image_path(image_path)?;
    let facts = signed_image_facts(&image_path)?;
    let catalogue = render_catalogue(&facts, host, port, installer_path);
    UpdateCatalogue::parse(&catalogue).map_err(|error| match error {
        anodrel_update_catalogue::UpdateCatalogueError::InstallerLocationInvalid => {
            UpdateCatalogueCreateError::LocationInvalid
        }
        _ => UpdateCatalogueCreateError::CatalogueInvalid,
    })?;
    output::write_new(output_path, catalogue.as_bytes())
}

struct CatalogueFacts {
    application_id: String,
    package_version: PackageVersion,
    publisher: [u8; 32],
    byte_length: u64,
    digest: [u8; 32],
}

fn signed_image_facts(path: &Path) -> Result<CatalogueFacts, UpdateCatalogueCreateError> {
    // The resource mapping prevents mutation while both the embedded manifest
    // and the measured installer bytes are read. It remains alive through the
    // descriptor read below, so neither view can silently describe another file.
    let image =
        verify_locked_installer_image(path).map_err(UpdateCatalogueCreateError::ImageInvalid)?;
    let signer =
        verify_embedded_signature(path).map_err(UpdateCatalogueCreateError::SignerInvalid)?;
    let manifest = image.manifest();
    if !manifest.matches_publisher_fingerprint(signer.as_bytes()) {
        return Err(UpdateCatalogueCreateError::ImageInvalid(
            anodrel_windows_installer::InstallerImageError::PublisherMismatch,
        ));
    }
    let descriptor = measured_descriptor(path)?;
    let facts = CatalogueFacts {
        application_id: manifest.application_id().to_owned(),
        package_version: manifest.package_version(),
        publisher: signer.as_bytes(),
        byte_length: descriptor.byte_length,
        digest: descriptor.digest,
    };
    drop(image);
    Ok(facts)
}

struct MeasuredDescriptor {
    byte_length: u64,
    digest: [u8; 32],
}

fn measured_descriptor(path: &Path) -> Result<MeasuredDescriptor, UpdateCatalogueCreateError> {
    let mut file = File::open(path).map_err(|_| UpdateCatalogueCreateError::InputReadFailed)?;
    let maximum = usize::try_from(MAX_UPDATE_IMAGE_BYTES)
        .map_err(|_| UpdateCatalogueCreateError::InputReadFailed)?;
    let (digest, byte_length) = sha256::digest_reader_limited(&mut file, maximum)
        .map_err(|_| UpdateCatalogueCreateError::InputReadFailed)?
        .ok_or(UpdateCatalogueCreateError::InputReadFailed)?;
    Ok(MeasuredDescriptor {
        byte_length: u64::try_from(byte_length)
            .map_err(|_| UpdateCatalogueCreateError::InputReadFailed)?,
        digest,
    })
}

fn canonical_image_path(path: &Path) -> Result<PathBuf, UpdateCatalogueCreateError> {
    if !path.is_absolute() {
        return Err(UpdateCatalogueCreateError::InputInvalid);
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| UpdateCatalogueCreateError::InputReadFailed)?;
    if !metadata.is_file() || is_link_like(&metadata) {
        return Err(UpdateCatalogueCreateError::InputInvalid);
    }
    if !(1..=MAX_UPDATE_IMAGE_BYTES).contains(&metadata.len()) {
        return Err(UpdateCatalogueCreateError::InputReadFailed);
    }
    fs::canonicalize(path).map_err(|_| UpdateCatalogueCreateError::InputReadFailed)
}

fn render_catalogue(facts: &CatalogueFacts, host: &str, port: u16, installer_path: &str) -> String {
    JsonValue::Object(
        [
            (
                "formatVersion".to_owned(),
                object([
                    ("major", JsonValue::Number("1".to_owned())),
                    ("minor", JsonValue::Number("0".to_owned())),
                ]),
            ),
            (
                "applicationId".to_owned(),
                JsonValue::String(facts.application_id.clone()),
            ),
            (
                "packageVersion".to_owned(),
                object([
                    (
                        "major",
                        JsonValue::Number(facts.package_version.major().to_string()),
                    ),
                    (
                        "minor",
                        JsonValue::Number(facts.package_version.minor().to_string()),
                    ),
                    (
                        "patch",
                        JsonValue::Number(facts.package_version.patch().to_string()),
                    ),
                ]),
            ),
            (
                "publisher".to_owned(),
                object([(
                    "leafCertificateSha256",
                    JsonValue::String(sha256::to_lower_hex(&facts.publisher)),
                )]),
            ),
            (
                "installer".to_owned(),
                object([
                    (
                        "origin",
                        object([
                            ("host", JsonValue::String(host.to_owned())),
                            ("port", JsonValue::Number(port.to_string())),
                        ]),
                    ),
                    ("path", JsonValue::String(installer_path.to_owned())),
                    (
                        "byteLength",
                        JsonValue::Number(facts.byte_length.to_string()),
                    ),
                    (
                        "sha256",
                        JsonValue::String(sha256::to_lower_hex(&facts.digest)),
                    ),
                ]),
            ),
        ]
        .into_iter()
        .collect(),
    )
    .to_json()
}

fn object<const N: usize>(fields: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        metadata.file_attributes() & 0x0400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use anodrel_update_catalogue::UpdateCatalogue;
    use anodrel_windows_installer::PackageVersion;

    use super::{CatalogueFacts, render_catalogue};

    #[test]
    fn rendered_catalogue_comes_from_measured_release_facts() {
        let catalogue = render_catalogue(
            &CatalogueFacts {
                application_id: "org.anodrel.product-fixture".to_owned(),
                package_version: PackageVersion::new(4, 2, 9),
                publisher: [0xA1; 32],
                byte_length: 123_456,
                digest: [0xB2; 32],
            },
            "updates.example.test",
            443,
            "/releases/4.2.9/fixture.exe",
        );
        let parsed = UpdateCatalogue::parse(&catalogue).expect("derived catalogue parses");

        assert_eq!(parsed.application_id(), "org.anodrel.product-fixture");
        assert_eq!(parsed.package_version(), PackageVersion::new(4, 2, 9));
        assert_eq!(parsed.installer().byte_length(), 123_456);
        assert!(parsed.installer().matches_descriptor(123_456, [0xB2; 32]));
    }

    #[test]
    fn an_invalid_publication_location_cannot_be_rendered_as_a_catalogue() {
        let catalogue = render_catalogue(
            &CatalogueFacts {
                application_id: "org.anodrel.product-fixture".to_owned(),
                package_version: PackageVersion::new(1, 0, 1),
                publisher: [0xA1; 32],
                byte_length: 1,
                digest: [0xB2; 32],
            },
            "updates.example.test",
            443,
            "/releases/unsafe path.exe",
        );

        assert!(UpdateCatalogue::parse(&catalogue).is_err());
    }
}
