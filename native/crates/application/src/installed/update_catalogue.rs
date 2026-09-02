//! Exact installed-policy values for one attached update-catalogue location.

use std::fmt;

use anodrel_network::NetworkOrigin;

/// Maximum ASCII request-path bytes in one installed update catalogue location.
pub const MAX_UPDATE_CATALOGUE_PATH_BYTES: usize = 512;

/// One exact signed HTTPS location for an attached update catalogue.
///
/// This is private native-host policy. It never grants application network
/// authority and must not be serialized to a renderer or protocol response.
pub struct UpdateCatalogueLocation {
    origin: NetworkOrigin,
    request_path: String,
}

/// A proposed update-catalogue location did not meet the exact policy grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateCatalogueLocationError {
    /// The proposed attached-CMS request path was not canonical and bounded.
    RequestPathInvalid,
}

impl fmt::Display for UpdateCatalogueLocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the update catalogue request path is invalid")
    }
}

impl std::error::Error for UpdateCatalogueLocationError {}

impl UpdateCatalogueLocation {
    /// Creates one exact HTTPS catalogue location from validated host policy.
    pub fn new(
        origin: NetworkOrigin,
        request_path: impl Into<String>,
    ) -> Result<Self, UpdateCatalogueLocationError> {
        let request_path = request_path.into();
        is_valid_request_path(&request_path)
            .then_some(Self {
                origin,
                request_path,
            })
            .ok_or(UpdateCatalogueLocationError::RequestPathInvalid)
    }

    /// Returns the exact TLS origin selected by signed installed policy.
    #[must_use]
    pub fn origin(&self) -> &NetworkOrigin {
        &self.origin
    }

    /// Returns the exact attached-CMS request path selected by installed policy.
    #[must_use]
    pub fn request_path(&self) -> &str {
        &self.request_path
    }
}

impl fmt::Debug for UpdateCatalogueLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UpdateCatalogueLocation(..)")
    }
}

fn is_valid_request_path(path: &str) -> bool {
    (1..=MAX_UPDATE_CATALOGUE_PATH_BYTES).contains(&path.len())
        && path.starts_with('/')
        && path.ends_with(".p7s")
        && !path.contains("//")
        && path
            .split('/')
            .skip(1)
            .all(|component| !component.is_empty() && component != "." && component != "..")
        && path.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~')
        })
}

#[cfg(test)]
mod tests {
    use anodrel_network::NetworkOrigin;

    use super::{UpdateCatalogueLocation, UpdateCatalogueLocationError};

    #[test]
    fn accepts_only_canonical_attached_catalogue_paths() {
        let origin = NetworkOrigin::new("updates.example.test", 443).expect("origin is valid");
        let location = UpdateCatalogueLocation::new(origin, "/anodrel/stable.p7s")
            .expect("canonical path is valid");
        assert_eq!(location.request_path(), "/anodrel/stable.p7s");
        assert_eq!(location.origin().hostname(), "updates.example.test");
        assert_eq!(format!("{location:?}"), "UpdateCatalogueLocation(..)");
    }

    #[test]
    fn rejects_noncanonical_or_non_catalogue_paths() {
        let origin = NetworkOrigin::new("updates.example.test", 443).expect("origin is valid");
        for path in [
            "catalogue.p7s",
            "/catalogue.exe",
            "/catalogue.P7S",
            "/catalogues//stable.p7s",
            "/catalogues/../stable.p7s",
            "/catalogues/stable.p7s?channel=stable",
        ] {
            assert!(
                matches!(
                    UpdateCatalogueLocation::new(origin.clone(), path),
                    Err(UpdateCatalogueLocationError::RequestPathInvalid)
                ),
                "{path} must fail"
            );
        }
    }
}
