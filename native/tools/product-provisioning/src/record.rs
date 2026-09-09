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
mod tests;
