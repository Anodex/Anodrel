//! Exact version-1 release-plan field parsing.

use std::collections::{BTreeMap, BTreeSet};

use anodrel_application::{StartMenuName, UpdateCatalogueLocation};
use anodrel_json::JsonValue;
use anodrel_network::NetworkOrigin;
use anodrel_windows_installer::ProductMetadata;

use super::PlanFormatVersion;
use crate::ReleaseManifestAuthorError;

pub(super) fn parse_format_version(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<PlanFormatVersion, ReleaseManifestAuthorError> {
    exact_fields(fields, &["major", "minor"])?;
    match (
        required_u16(fields, "major")?,
        required_u16(fields, "minor")?,
    ) {
        (1, 0) => Ok(PlanFormatVersion::Base),
        (1, 1) => Ok(PlanFormatVersion::Catalogue),
        (1, 2) => Ok(PlanFormatVersion::ProductMetadata),
        (1, 3) => Ok(PlanFormatVersion::ProductRegistration),
        (1, 4) => Ok(PlanFormatVersion::ProductLauncher),
        _ => Err(ReleaseManifestAuthorError::PlanInvalid),
    }
}

pub(super) fn parse_launcher_path(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<String, ReleaseManifestAuthorError> {
    exact_fields(fields, &["path"])?;
    Ok(required_string(fields, "path")?.to_owned())
}

pub(super) fn parse_product_metadata(
    fields: &BTreeMap<String, JsonValue>,
    requires_start_menu_name: bool,
) -> Result<(ProductMetadata, Option<StartMenuName>), ReleaseManifestAuthorError> {
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
    .map_err(|_| ReleaseManifestAuthorError::PlanInvalid)?;
    let start_menu_name = requires_start_menu_name
        .then(|| {
            StartMenuName::new(required_string(fields, "startMenuName")?)
                .map_err(|_| ReleaseManifestAuthorError::PlanInvalid)
        })
        .transpose()?;
    Ok((metadata, start_menu_name))
}

pub(super) fn parse_capabilities(
    value: Option<&JsonValue>,
) -> Result<Vec<String>, ReleaseManifestAuthorError> {
    let Some(JsonValue::Array(values)) = value else {
        return Err(ReleaseManifestAuthorError::PlanInvalid);
    };
    let mut names = BTreeSet::new();
    for value in values {
        names.insert(
            value
                .as_string()
                .ok_or(ReleaseManifestAuthorError::PlanInvalid)?
                .to_owned(),
        );
    }
    (names.len() == values.len())
        .then_some(names.into_iter().collect())
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}

pub(super) fn parse_network_origins(
    value: Option<&JsonValue>,
) -> Result<Vec<NetworkOrigin>, ReleaseManifestAuthorError> {
    let Some(JsonValue::Array(values)) = value else {
        return Err(ReleaseManifestAuthorError::PlanInvalid);
    };
    values
        .iter()
        .map(|value| {
            let fields = value
                .as_object()
                .ok_or(ReleaseManifestAuthorError::PlanInvalid)?;
            exact_fields(fields, &["host", "port"])?;
            NetworkOrigin::new(
                required_string(fields, "host")?,
                required_u16(fields, "port")?,
            )
            .map_err(|_| ReleaseManifestAuthorError::PlanInvalid)
        })
        .collect()
}

pub(super) fn parse_update_catalogue(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<UpdateCatalogueLocation, ReleaseManifestAuthorError> {
    exact_fields(fields, &["origin", "path"])?;
    let origin = required_object(fields, "origin")?;
    exact_fields(origin, &["host", "port"])?;
    let origin = NetworkOrigin::new(
        required_string(origin, "host")?,
        required_u16(origin, "port")?,
    )
    .map_err(|_| ReleaseManifestAuthorError::PlanInvalid)?;
    UpdateCatalogueLocation::new(origin, required_string(fields, "path")?)
        .map_err(|_| ReleaseManifestAuthorError::PlanInvalid)
}

pub(super) fn exact_fields(
    fields: &BTreeMap<String, JsonValue>,
    expected: &[&str],
) -> Result<(), ReleaseManifestAuthorError> {
    (fields.len() == expected.len() && expected.iter().all(|name| fields.contains_key(*name)))
        .then_some(())
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}

pub(super) fn required_object<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, ReleaseManifestAuthorError> {
    fields
        .get(name)
        .and_then(JsonValue::as_object)
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}

pub(super) fn required_string<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, ReleaseManifestAuthorError> {
    fields
        .get(name)
        .and_then(JsonValue::as_string)
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}

pub(super) fn required_u16(
    fields: &BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<u16, ReleaseManifestAuthorError> {
    fields
        .get(name)
        .and_then(JsonValue::as_u16)
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}
