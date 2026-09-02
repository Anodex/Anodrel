//! Strict signed product metadata parsing for release manifests.

use std::collections::BTreeMap;

use anodrel_application::StartMenuName;
use anodrel_json::JsonValue;

use crate::ReleaseManifestError;

use super::{ProductMetadata, exact_fields, required_string};

/// Parses the exact product metadata shape for one release-manifest version.
pub(super) fn parse(
    fields: &BTreeMap<String, JsonValue>,
    requires_start_menu_name: bool,
) -> Result<(ProductMetadata, Option<StartMenuName>), ReleaseManifestError> {
    let expected_fields = if requires_start_menu_name {
        &["displayName", "publisherName", "startMenuName"][..]
    } else {
        &["displayName", "publisherName"][..]
    };
    exact_fields(fields, expected_fields)?;
    let metadata = ProductMetadata::new(
        required_string(fields, "displayName")?,
        required_string(fields, "publisherName")?,
    )
    .map_err(|_| ReleaseManifestError::ProductMetadataInvalid)?;
    let start_menu_name = requires_start_menu_name
        .then(|| {
            StartMenuName::new(required_string(fields, "startMenuName")?)
                .map_err(|_| ReleaseManifestError::ProductMetadataInvalid)
        })
        .transpose()?;
    Ok((metadata, start_menu_name))
}
