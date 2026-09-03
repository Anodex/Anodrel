//! Composition of the one machine-policy record this helper may write.
//!
//! The record shape is defined by `docs/LAUNCH.md`. This module builds it from
//! measured facts only — a recomputed executable digest and an Authenticode
//! leaf fingerprint Windows actually accepted — and then re-validates the result
//! through the same parser the native host uses. A record the helper could not
//! itself validate is never written.

use std::{
    fmt,
    fs::File,
    io,
    path::{Path, PathBuf},
};

use anodrel_application::{
    InstalledApplication, InstalledApplicationError, MAX_EXECUTABLE_BYTES, sha256,
};
use anodrel_json::JsonValue;
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

use crate::fixture;

/// Builds and validates the fixture's record for one staged package root.
pub fn compose(
    package_root: &Path,
    executable: &Path,
    launcher: &Path,
) -> Result<String, RecordError> {
    let package_root = std::fs::canonicalize(package_root).map_err(RecordError::Io)?;
    let executable = inspect_image(executable).map_err(RecordError::ExecutableInvalid)?;
    let launcher = inspect_image(launcher).map_err(RecordError::LauncherInvalid)?;
    require_matching_publishers(&executable, &launcher)?;
    let record = render(
        &package_root,
        &executable.digest,
        &launcher.digest,
        &sha256::to_lower_hex(&executable.publisher),
    );

    // Fail before writing rather than leaving a machine-policy value the host
    // would reject at launch time.
    InstalledApplication::load_from_trusted_record(&record, fixture::APPLICATION_ID)
        .map_err(RecordError::Record)?;
    Ok(record)
}

struct ImageFacts {
    digest: String,
    publisher: [u8; 32],
}

fn inspect_image(image: &Path) -> Result<ImageFacts, ImageInspectionError> {
    let mut file = File::open(image).map_err(ImageInspectionError::Io)?;
    let (digest, _) = sha256::digest_reader_limited(&mut file, MAX_EXECUTABLE_BYTES)
        .map_err(ImageInspectionError::Io)?
        .ok_or(ImageInspectionError::TooLarge)?;
    let publisher = verify_embedded_signature(image)
        .map_err(ImageInspectionError::Signature)?
        .as_bytes();
    Ok(ImageFacts {
        digest: sha256::to_lower_hex(&digest),
        publisher,
    })
}

fn require_matching_publishers(
    executable: &ImageFacts,
    launcher: &ImageFacts,
) -> Result<(), RecordError> {
    (executable.publisher == launcher.publisher)
        .then_some(())
        .ok_or(RecordError::PublisherMismatch)
}

/// Renders the strict record JSON for measured facts.
fn render(
    package_root: &Path,
    executable_digest: &str,
    launcher_digest: &str,
    publisher_digest: &str,
) -> String {
    let capabilities = fixture::CAPABILITIES
        .iter()
        .map(|capability| JsonValue::String((*capability).to_owned()))
        .collect();
    JsonValue::Object(
        [
            (
                "recordVersion".to_owned(),
                object([
                    ("major", JsonValue::Number("1".to_owned())),
                    ("minor", JsonValue::Number("23".to_owned())),
                ]),
            ),
            (
                "applicationId".to_owned(),
                JsonValue::String(fixture::APPLICATION_ID.to_owned()),
            ),
            (
                "packageRoot".to_owned(),
                JsonValue::String(package_root.display().to_string()),
            ),
            (
                "executable".to_owned(),
                object([
                    (
                        "path",
                        JsonValue::String(fixture::EXECUTABLE_PATH.to_owned()),
                    ),
                    ("sha256", JsonValue::String(executable_digest.to_owned())),
                ]),
            ),
            (
                "publisher".to_owned(),
                object([(
                    "leafCertificateSha256",
                    JsonValue::String(publisher_digest.to_owned()),
                )]),
            ),
            ("capabilities".to_owned(), JsonValue::Array(capabilities)),
            ("networkOrigins".to_owned(), JsonValue::Array(Vec::new())),
            (
                "updateCatalogue".to_owned(),
                object([
                    (
                        "origin",
                        object([
                            (
                                "host",
                                JsonValue::String(fixture::UPDATE_CATALOGUE_HOST.to_owned()),
                            ),
                            (
                                "port",
                                JsonValue::Number(fixture::UPDATE_CATALOGUE_PORT.to_string()),
                            ),
                        ]),
                    ),
                    (
                        "path",
                        JsonValue::String(fixture::UPDATE_CATALOGUE_PATH.to_owned()),
                    ),
                ]),
            ),
            (
                "product".to_owned(),
                object([
                    (
                        "displayName",
                        JsonValue::String(fixture::DISPLAY_NAME.to_owned()),
                    ),
                    (
                        "publisherName",
                        JsonValue::String(fixture::PUBLISHER_NAME.to_owned()),
                    ),
                    (
                        "startMenuName",
                        JsonValue::String(fixture::START_MENU_NAME.to_owned()),
                    ),
                ]),
            ),
            (
                "launcher".to_owned(),
                object([
                    ("path", JsonValue::String(fixture::LAUNCHER_PATH.to_owned())),
                    ("sha256", JsonValue::String(launcher_digest.to_owned())),
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

/// The canonical path a caller supplied, kept private to this helper.
pub fn canonical_package_root(value: &str) -> io::Result<PathBuf> {
    let path = std::fs::canonicalize(value)?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the fixture package root is not a directory",
        ))
    }
}

/// A safe failure category while composing the fixture record.
#[derive(Debug)]
pub enum RecordError {
    Io(io::Error),
    ExecutableInvalid(ImageInspectionError),
    LauncherInvalid(ImageInspectionError),
    PublisherMismatch,
    Record(InstalledApplicationError),
}

impl fmt::Display for RecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io(_) => "the fixture package root could not be read",
            Self::ExecutableInvalid(ImageInspectionError::Io(_)) => {
                "the staged fixture executable could not be read"
            }
            Self::ExecutableInvalid(ImageInspectionError::TooLarge) => {
                "the staged fixture executable exceeds its limit"
            }
            Self::ExecutableInvalid(ImageInspectionError::Signature(_)) => {
                "Windows did not accept the fixture executable signature"
            }
            Self::LauncherInvalid(ImageInspectionError::Io(_)) => {
                "the staged product launcher could not be read"
            }
            Self::LauncherInvalid(ImageInspectionError::TooLarge) => {
                "the staged product launcher exceeds its limit"
            }
            Self::LauncherInvalid(ImageInspectionError::Signature(_)) => {
                "Windows did not accept the staged product launcher signature"
            }
            Self::PublisherMismatch => {
                "the staged product launcher publisher does not match the fixture executable"
            }
            Self::Record(_) => "the composed fixture record did not validate",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RecordError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::ExecutableInvalid(error) | Self::LauncherInvalid(error) => Some(error),
            Self::Record(error) => Some(error),
            Self::PublisherMismatch => None,
        }
    }
}

/// A safe failure while measuring one fixture image before policy publication.
#[derive(Debug)]
pub enum ImageInspectionError {
    Io(io::Error),
    TooLarge,
    Signature(SignatureError),
}

impl fmt::Display for ImageInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Io(_) => "the image could not be read",
            Self::TooLarge => "the image exceeds its limit",
            Self::Signature(_) => "Windows did not accept the image signature",
        })
    }
}

impl std::error::Error for ImageInspectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Signature(error) => Some(error),
            Self::TooLarge => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use anodrel_application::{InstalledApplication, sha256};
    use anodrel_json::JsonValue;

    use super::{
        ImageFacts, ImageInspectionError, RecordError, compose, fixture, render,
        require_matching_publishers,
    };

    const DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const LAUNCHER_DIGEST: &str =
        "3f79bb7b435b05321651daefd374cd21b4e6a0a54f9f4dbb85dfbb6b6c4b6bc0";
    const FINGERPRINT: &str = "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585";

    /// A staged fixture package that removes itself when the test ends.
    struct StagedPackage(PathBuf);

    impl StagedPackage {
        /// Stages a package and writes placeholder executable bytes.
        ///
        /// The bytes need not be a real image: every check under test runs
        /// before Windows would load one.
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("anodrel-record-test-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            crate::package::stage(&root).expect("the fixture package stages");
            std::fs::write(Self::executable_at(&root), b"placeholder fixture image")
                .expect("the placeholder executable is written");
            std::fs::write(Self::launcher_at(&root), b"placeholder host image")
                .expect("the placeholder launcher is written");
            Self(root)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        fn executable(&self) -> PathBuf {
            std::fs::canonicalize(Self::executable_at(&self.0))
                .expect("the placeholder executable resolves")
        }

        fn executable_at(root: &Path) -> PathBuf {
            root.join("bin").join(fixture::EXECUTABLE_FILE_NAME)
        }

        fn launcher(&self) -> PathBuf {
            std::fs::canonicalize(Self::launcher_at(&self.0))
                .expect("the placeholder launcher resolves")
        }

        fn launcher_at(root: &Path) -> PathBuf {
            root.join("bin").join(fixture::LAUNCHER_FILE_NAME)
        }

        /// The digest the record parser will recompute for this package.
        fn executable_digest(&self) -> String {
            let bytes = std::fs::read(self.executable()).expect("the placeholder executable reads");
            sha256::to_lower_hex(&sha256::digest(&bytes))
        }

        fn launcher_digest(&self) -> String {
            let bytes = std::fs::read(self.launcher()).expect("the placeholder launcher reads");
            sha256::to_lower_hex(&sha256::digest(&bytes))
        }
    }

    impl Drop for StagedPackage {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn composition_refuses_an_executable_windows_will_not_vouch_for() {
        // This is the guard that keeps an unsigned build out of machine policy.
        // `main` reaches `write_record` only through a successful `compose`, so
        // failing here means nothing is written.
        let package = StagedPackage::new("unsigned");
        assert!(matches!(
            compose(package.root(), &package.executable(), &package.launcher()),
            Err(RecordError::ExecutableInvalid(
                ImageInspectionError::Signature(_)
            ))
        ));
    }

    #[test]
    fn composition_refuses_an_executable_that_is_not_there() {
        let package = StagedPackage::new("absent");
        assert!(matches!(
            compose(
                package.root(),
                &package.root().join("bin").join("absent.exe"),
                &package.launcher(),
            ),
            Err(RecordError::ExecutableInvalid(ImageInspectionError::Io(_)))
        ));
    }

    #[test]
    fn a_record_whose_digest_does_not_match_its_package_is_rejected() {
        // `compose` validates through this same parser before writing, so a
        // stale or substituted binary can never reach machine policy.
        let package = StagedPackage::new("digest");
        let record = render(
            &std::fs::canonicalize(package.root()).expect("the package root resolves"),
            DIGEST,
            &package.launcher_digest(),
            FINGERPRINT,
        );

        assert!(matches!(
            InstalledApplication::load_from_trusted_record(&record, fixture::APPLICATION_ID),
            Err(anodrel_application::InstalledApplicationError::ExecutableDigestMismatch)
        ));
    }

    #[test]
    fn a_record_offered_under_another_identity_is_rejected() {
        let package = StagedPackage::new("identity");
        let record = render(
            &std::fs::canonicalize(package.root()).expect("the package root resolves"),
            &package.executable_digest(),
            &package.launcher_digest(),
            FINGERPRINT,
        );

        // The same record validates for its own identity and fails for another,
        // which is what stops provisioning from redirecting an existing key.
        let installed =
            InstalledApplication::load_from_trusted_record(&record, fixture::APPLICATION_ID)
                .expect("the fixture record validates for its own identity");
        let mut launcher = std::fs::File::open(package.launcher()).expect("launcher opens");
        installed
            .revalidate_product_launcher(
                installed
                    .product_launcher_path()
                    .expect("record retains the launcher"),
                &mut launcher,
            )
            .expect("the fixture launcher revalidates through its record digest");
        assert!(matches!(
            InstalledApplication::load_from_trusted_record(&record, "org.anodrel.sample"),
            Err(anodrel_application::InstalledApplicationError::ApplicationIdentityMismatch)
        ));
    }

    #[test]
    fn a_composed_record_carries_exactly_the_documented_version_and_fields() {
        let record = render(
            std::path::Path::new("C:\\fixture"),
            DIGEST,
            LAUNCHER_DIGEST,
            FINGERPRINT,
        );
        let value = JsonValue::parse(&record).expect("the composed record is JSON");
        let fields = value.as_object().expect("the composed record is an object");

        let mut names = fields.keys().cloned().collect::<Vec<_>>();
        names.sort();
        assert_eq!(
            names,
            [
                "applicationId",
                "capabilities",
                "executable",
                "launcher",
                "networkOrigins",
                "packageRoot",
                "product",
                "publisher",
                "recordVersion",
                "updateCatalogue",
            ]
        );
        assert_eq!(
            fields.get("applicationId").and_then(JsonValue::as_string),
            Some(fixture::APPLICATION_ID)
        );
        assert_eq!(
            fields
                .get("recordVersion")
                .and_then(JsonValue::as_object)
                .and_then(|version| version.get("minor"))
                .and_then(JsonValue::as_u16),
            Some(23)
        );
    }

    #[test]
    fn a_composed_record_grants_only_the_fixtures_three_capabilities() {
        let record = render(
            std::path::Path::new("C:\\fixture"),
            DIGEST,
            LAUNCHER_DIGEST,
            FINGERPRINT,
        );
        let value = JsonValue::parse(&record).expect("the composed record is JSON");
        let Some(JsonValue::Array(capabilities)) = value
            .as_object()
            .and_then(|fields| fields.get("capabilities"))
        else {
            panic!("the composed record has a capability array");
        };

        let granted = capabilities
            .iter()
            .filter_map(JsonValue::as_string)
            .collect::<Vec<_>>();
        assert_eq!(granted, fixture::CAPABILITIES);
    }

    #[test]
    fn a_composed_record_stays_inside_the_record_size_limit() {
        let record = render(
            std::path::Path::new(&format!("C:\\{}", "fixture".repeat(30))),
            DIGEST,
            LAUNCHER_DIGEST,
            FINGERPRINT,
        );
        assert!(record.len() < anodrel_application::MAX_INSTALL_RECORD_BYTES);
    }

    #[test]
    fn differently_signed_child_and_launcher_cannot_share_one_record() {
        let executable = ImageFacts {
            digest: DIGEST.to_owned(),
            publisher: [1; 32],
        };
        let launcher = ImageFacts {
            digest: LAUNCHER_DIGEST.to_owned(),
            publisher: [2; 32],
        };
        assert!(matches!(
            require_matching_publishers(&executable, &launcher),
            Err(RecordError::PublisherMismatch)
        ));
    }
}
