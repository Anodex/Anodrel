//! Canonical-path package loading and bounded text validation.

use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use crate::{
    ApplicationError, ApplicationIdentity, ApplicationManifest, MAX_CONTENT_BYTES,
    MAX_MANIFEST_BYTES, sha256,
};

const MAX_TEXT_SCALARS: usize = 4_096;
const MAX_TEXT_LINES: usize = 128;
const MAX_TEXT_LINE_SCALARS: usize = 160;

/// Facts about content that has already passed containment and digest checks.
///
/// Every field here is safe for a host surface to display: the path is the
/// manifest's declared relative path, never the resolved filesystem location,
/// and the digest is the value that was verified rather than one recomputed on
/// demand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedContent {
    format: String,
    path: String,
    digest: String,
    byte_length: usize,
}

impl VerifiedContent {
    /// Returns the declared content format.
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Returns the package-relative content path declared by the manifest.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the verified SHA-256 digest as lower-case hexadecimal.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Returns the number of content bytes that were hashed.
    pub fn byte_length(&self) -> usize {
        self.byte_length
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationPackage {
    identity: ApplicationIdentity,
    content: VerifiedContent,
    text: String,
}

impl ApplicationPackage {
    /// Loads and fully validates an application package before returning any
    /// application-controlled text to a caller.
    pub fn load(manifest_path: impl AsRef<Path>) -> Result<Self, ApplicationError> {
        let manifest_path = fs::canonicalize(manifest_path)?;
        let package_root = manifest_path
            .parent()
            .ok_or(ApplicationError::InvalidManifest)?;
        let package_root = fs::canonicalize(package_root)?;

        let manifest_bytes = read_limited(&manifest_path, MAX_MANIFEST_BYTES)?;
        let manifest_text =
            std::str::from_utf8(&manifest_bytes).map_err(|_| ApplicationError::InvalidManifest)?;
        let manifest = ApplicationManifest::parse(manifest_text)?;

        let content_path = canonical_content_path(&package_root, &manifest)?;
        let content_bytes = read_limited(&content_path, MAX_CONTENT_BYTES)?;
        if sha256::digest(&content_bytes) != *manifest.content_digest() {
            return Err(ApplicationError::ContentDigestMismatch);
        }
        let byte_length = content_bytes.len();
        let text = String::from_utf8(content_bytes).map_err(|_| ApplicationError::InvalidText)?;
        validate_text(&text)?;

        Ok(Self {
            identity: manifest.identity().clone(),
            content: VerifiedContent {
                format: crate::TEXT_CONTENT_FORMAT.to_owned(),
                path: manifest.content_path().to_owned(),
                digest: sha256::to_lower_hex(manifest.content_digest()),
                byte_length,
            },
            text,
        })
    }

    pub fn identity(&self) -> &ApplicationIdentity {
        &self.identity
    }

    /// Returns the verified facts about the package's content.
    pub fn content(&self) -> &VerifiedContent {
        &self.content
    }

    /// Returns text that has passed the documented size and character limits.
    pub fn text(&self) -> &str {
        &self.text
    }
}

fn canonical_content_path(
    package_root: &Path,
    manifest: &ApplicationManifest,
) -> Result<PathBuf, ApplicationError> {
    let content_path = fs::canonicalize(package_root.join(manifest.content_path()))?;
    if content_path.starts_with(package_root) {
        Ok(content_path)
    } else {
        Err(ApplicationError::ContentOutsidePackage)
    }
}

fn read_limited(path: &Path, maximum: usize) -> Result<Vec<u8>, ApplicationError> {
    let file = File::open(path)?;
    let mut reader = file.take((maximum + 1) as u64);
    let mut contents = Vec::with_capacity(maximum.min(4_096));
    reader.read_to_end(&mut contents)?;
    if contents.len() > maximum {
        return if maximum == MAX_MANIFEST_BYTES {
            Err(ApplicationError::ManifestTooLarge)
        } else {
            Err(ApplicationError::ContentTooLarge)
        };
    }
    Ok(contents)
}

fn validate_text(text: &str) -> Result<(), ApplicationError> {
    if text.chars().count() > MAX_TEXT_SCALARS
        || text
            .chars()
            .any(|character| character.is_control() && character != '\n')
    {
        return Err(ApplicationError::InvalidText);
    }

    let mut line_count = 0;
    for line in text.split('\n') {
        line_count += 1;
        if line.chars().count() > MAX_TEXT_LINE_SCALARS {
            return Err(ApplicationError::InvalidText);
        }
    }
    if line_count > MAX_TEXT_LINES {
        return Err(ApplicationError::InvalidText);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::ApplicationPackage;
    use crate::{ApplicationError, sha256};

    fn fixture(content: &[u8], declared_digest: &str) -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "anodrel-application-package-{}-{unique}",
            std::process::id()
        ));
        let content_path = root.join("content").join("main.txt");
        fs::create_dir_all(content_path.parent().expect("content has parent"))
            .expect("fixture directory is created");
        fs::write(&content_path, content).expect("fixture content is written");
        fs::write(
            root.join("anodrel.application.json"),
            format!(
                r#"{{
                    "manifestVersion": {{"major": 1, "minor": 0}},
                    "applicationId": "org.anodrel.sample",
                    "displayName": "Anodrel Sample",
                    "content": {{
                        "format": "anodrel.text.v1",
                        "path": "content/main.txt",
                        "sha256": "{declared_digest}"
                    }}
                }}"#
            ),
        )
        .expect("fixture manifest is written");
        (root.join("anodrel.application.json"), root)
    }

    fn digest(content: &[u8]) -> String {
        sha256::lower_hex(&sha256::digest(content))
    }

    fn remove_fixture(root: &Path) {
        fs::remove_dir_all(root).expect("fixture directory is removed");
    }

    #[test]
    fn loads_digest_verified_text_with_manifest_identity() {
        let content = b"Hello from the verified package.\n";
        let (manifest, root) = fixture(content, &digest(content));

        let package = ApplicationPackage::load(&manifest).expect("package is valid");

        assert_eq!(package.identity().application_id(), "org.anodrel.sample");
        assert_eq!(package.identity().display_name(), "Anodrel Sample");
        assert_eq!(package.text(), "Hello from the verified package.\n");
        remove_fixture(&root);
    }

    #[test]
    fn rejects_tampered_content_before_returning_text() {
        let (manifest, root) = fixture(b"changed", &digest(b"original"));

        assert!(matches!(
            ApplicationPackage::load(&manifest),
            Err(ApplicationError::ContentDigestMismatch)
        ));
        remove_fixture(&root);
    }

    #[test]
    fn rejects_control_characters_even_when_the_digest_matches() {
        let content = b"not\tallowed";
        let (manifest, root) = fixture(content, &digest(content));

        assert!(matches!(
            ApplicationPackage::load(&manifest),
            Err(ApplicationError::InvalidText)
        ));
        remove_fixture(&root);
    }
}
