//! Exact release-manifest format-version selection.

use std::collections::BTreeMap;

use anodrel_json::JsonValue;

use crate::ReleaseManifestError;

use super::fields::{exact_fields, required_u16};

/// One supported strict release-manifest field set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FormatVersion {
    /// The original release shape with no update catalogue or product data.
    Base,
    /// The update-catalogue release shape.
    Catalogue,
    /// The product display-metadata release shape.
    ProductMetadata,
    /// The product-registration release shape with a Start-menu filename.
    ProductRegistration,
    /// The product-launch release shape with a verified host executable.
    ProductLauncher,
}

impl FormatVersion {
    /// Whether this release shape requires an update catalogue.
    pub(super) const fn has_update_catalogue(self) -> bool {
        !matches!(self, Self::Base)
    }

    /// Whether this release shape requires general product display metadata.
    pub(super) const fn has_product_metadata(self) -> bool {
        matches!(
            self,
            Self::ProductMetadata | Self::ProductRegistration | Self::ProductLauncher
        )
    }

    /// Whether this release shape requires a Windows Start-menu filename.
    pub(super) const fn has_start_menu_name(self) -> bool {
        matches!(self, Self::ProductRegistration | Self::ProductLauncher)
    }

    /// Whether this release carries a verified Windows host launcher.
    pub(super) const fn has_product_launcher(self) -> bool {
        matches!(self, Self::ProductLauncher)
    }
}

/// Parses one exact supported release-manifest version object.
pub(super) fn parse(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<FormatVersion, ReleaseManifestError> {
    exact_fields(fields, &["major", "minor"])?;
    match (
        required_u16(fields, "major")?,
        required_u16(fields, "minor")?,
    ) {
        (1, 0) => Ok(FormatVersion::Base),
        (1, 1) => Ok(FormatVersion::Catalogue),
        (1, 2) => Ok(FormatVersion::ProductMetadata),
        (1, 3) => Ok(FormatVersion::ProductRegistration),
        (1, 4) => Ok(FormatVersion::ProductLauncher),
        (1, _) => Err(ReleaseManifestError::VersionUnsupported),
        _ => Err(ReleaseManifestError::VersionUnsupported),
    }
}
