//! Signed product-launcher descriptor parsing for release manifests.

use anodrel_application::sha256;
use anodrel_json::JsonValue;

use crate::ReleaseManifestError;

use super::fields::{exact_fields, is_valid_executable_path, required_string};

/// One distinct Anodrel Windows host executable selected for product launch.
pub struct ProductLauncher {
    path: String,
    digest: [u8; 32],
}

impl ProductLauncher {
    /// Parses the strict signed launcher descriptor.
    pub(super) fn parse(
        fields: &std::collections::BTreeMap<String, JsonValue>,
    ) -> Result<Self, ReleaseManifestError> {
        exact_fields(fields, &["path", "sha256"])?;
        let path = required_string(fields, "path")?;
        if !is_valid_executable_path(path) {
            return Err(ReleaseManifestError::ExecutablePathInvalid);
        }
        let digest = sha256::parse_lower_hex(required_string(fields, "sha256")?)
            .ok_or(ReleaseManifestError::Invalid)?;
        Ok(Self {
            path: path.to_owned(),
            digest,
        })
    }

    /// Returns the contained launcher bundle path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Compares a calculated launcher digest without displaying it.
    #[must_use]
    pub fn matches_digest(&self, actual: [u8; 32]) -> bool {
        self.digest == actual
    }

    pub(crate) const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

impl std::fmt::Debug for ProductLauncher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProductLauncher")
            .field("path", &self.path)
            .field("digest", &"[redacted]")
            .finish()
    }
}
