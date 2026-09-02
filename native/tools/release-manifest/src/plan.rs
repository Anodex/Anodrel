//! Strict version-1 release-plan parsing and final-manifest rendering.

use std::collections::{BTreeMap, BTreeSet};

use anodrel_application::{StartMenuName, UpdateCatalogueLocation, sha256};
use anodrel_json::JsonValue;
use anodrel_network::NetworkOrigin;
use anodrel_windows_installer::ProductMetadata;

use crate::ReleaseManifestAuthorError;

/// The operator-selected facts that may enter a derived release manifest.
pub(super) struct ReleasePlan {
    package_version: [u16; 3],
    executable_path: String,
    publisher: [u8; 32],
    capabilities: Vec<String>,
    network_origins: Vec<NetworkOrigin>,
    update_catalogue: Option<UpdateCatalogueLocation>,
    product_metadata: Option<ProductMetadata>,
    start_menu_name: Option<StartMenuName>,
}

impl ReleasePlan {
    /// Parses one exact version-1 release plan.
    pub(super) fn parse(input: &str) -> Result<Self, ReleaseManifestAuthorError> {
        let root = JsonValue::parse(input).map_err(|_| ReleaseManifestAuthorError::PlanInvalid)?;
        let fields = root
            .as_object()
            .ok_or(ReleaseManifestAuthorError::PlanInvalid)?;
        let format_version = parse_format_version(required_object(fields, "formatVersion")?)?;
        let expected_fields = match format_version {
            PlanFormatVersion::Base => &[
                "formatVersion",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
            ][..],
            PlanFormatVersion::Catalogue => &[
                "formatVersion",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "updateCatalogue",
            ][..],
            PlanFormatVersion::ProductMetadata => &[
                "formatVersion",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "updateCatalogue",
                "product",
            ][..],
            PlanFormatVersion::ProductRegistration => &[
                "formatVersion",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "updateCatalogue",
                "product",
            ][..],
        };
        exact_fields(fields, expected_fields)?;
        let package = required_object(fields, "packageVersion")?;
        exact_fields(package, &["major", "minor", "patch"])?;
        let package_version = [
            required_u16(package, "major")?,
            required_u16(package, "minor")?,
            required_u16(package, "patch")?,
        ];
        let executable = required_object(fields, "executable")?;
        exact_fields(executable, &["path"])?;
        let executable_path = required_string(executable, "path")?.to_owned();
        let publisher = required_object(fields, "publisher")?;
        exact_fields(publisher, &["leafCertificateSha256"])?;
        let publisher =
            sha256::parse_lower_hex(required_string(publisher, "leafCertificateSha256")?)
                .ok_or(ReleaseManifestAuthorError::PlanInvalid)?;
        let capabilities = parse_capabilities(fields.get("capabilities"))?;
        let network_origins = parse_network_origins(fields.get("networkOrigins"))?;
        let update_catalogue = if format_version.has_update_catalogue() {
            Some(parse_update_catalogue(required_object(
                fields,
                "updateCatalogue",
            )?)?)
        } else {
            None
        };
        let (product_metadata, start_menu_name) = if format_version.has_product_metadata() {
            let (metadata, name) = parse_product_metadata(
                required_object(fields, "product")?,
                format_version.has_start_menu_name(),
            )?;
            (Some(metadata), name)
        } else {
            (None, None)
        };
        Ok(Self {
            package_version,
            executable_path,
            publisher,
            capabilities,
            network_origins,
            update_catalogue,
            product_metadata,
            start_menu_name,
        })
    }

    /// Returns the exact planned executable bundle path.
    pub(super) fn executable_path(&self) -> &str {
        &self.executable_path
    }

    /// Renders one final manifest from facts derived from checked bundle bytes.
    pub(super) fn render(
        &self,
        application_id: &str,
        executable_digest: [u8; 32],
        bundle: &[u8],
    ) -> String {
        let mut fields = BTreeMap::from([
            (
                "applicationId".to_owned(),
                JsonValue::String(application_id.to_owned()),
            ),
            (
                "capabilities".to_owned(),
                JsonValue::Array(
                    self.capabilities
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ),
            (
                "executable".to_owned(),
                JsonValue::Object(BTreeMap::from([
                    (
                        "path".to_owned(),
                        JsonValue::String(self.executable_path.clone()),
                    ),
                    (
                        "sha256".to_owned(),
                        JsonValue::String(sha256::to_lower_hex(&executable_digest)),
                    ),
                ])),
            ),
            (
                "formatVersion".to_owned(),
                JsonValue::Object(BTreeMap::from([
                    ("major".to_owned(), JsonValue::Number("1".to_owned())),
                    (
                        "minor".to_owned(),
                        JsonValue::Number(self.format_minor().to_string()),
                    ),
                ])),
            ),
            (
                "networkOrigins".to_owned(),
                JsonValue::Array(
                    self.network_origins
                        .iter()
                        .map(|origin| {
                            JsonValue::Object(BTreeMap::from([
                                (
                                    "host".to_owned(),
                                    JsonValue::String(origin.hostname().to_owned()),
                                ),
                                (
                                    "port".to_owned(),
                                    JsonValue::Number(origin.port().to_string()),
                                ),
                            ]))
                        })
                        .collect(),
                ),
            ),
            (
                "packageVersion".to_owned(),
                JsonValue::Object(BTreeMap::from([
                    (
                        "major".to_owned(),
                        JsonValue::Number(self.package_version[0].to_string()),
                    ),
                    (
                        "minor".to_owned(),
                        JsonValue::Number(self.package_version[1].to_string()),
                    ),
                    (
                        "patch".to_owned(),
                        JsonValue::Number(self.package_version[2].to_string()),
                    ),
                ])),
            ),
            (
                "payload".to_owned(),
                JsonValue::Object(BTreeMap::from([
                    (
                        "byteLength".to_owned(),
                        JsonValue::Number(bundle.len().to_string()),
                    ),
                    (
                        "sha256".to_owned(),
                        JsonValue::String(sha256::to_lower_hex(&sha256::digest(bundle))),
                    ),
                ])),
            ),
            (
                "publisher".to_owned(),
                JsonValue::Object(BTreeMap::from([(
                    "leafCertificateSha256".to_owned(),
                    JsonValue::String(sha256::to_lower_hex(&self.publisher)),
                )])),
            ),
        ]);
        if let Some(location) = &self.update_catalogue {
            fields.insert(
                "updateCatalogue".to_owned(),
                JsonValue::Object(BTreeMap::from([
                    (
                        "origin".to_owned(),
                        JsonValue::Object(BTreeMap::from([
                            (
                                "host".to_owned(),
                                JsonValue::String(location.origin().hostname().to_owned()),
                            ),
                            (
                                "port".to_owned(),
                                JsonValue::Number(location.origin().port().to_string()),
                            ),
                        ])),
                    ),
                    (
                        "path".to_owned(),
                        JsonValue::String(location.request_path().to_owned()),
                    ),
                ])),
            );
        }
        if let Some(product) = &self.product_metadata {
            let mut product_fields = BTreeMap::from([
                (
                    "displayName".to_owned(),
                    JsonValue::String(product.display_name().to_owned()),
                ),
                (
                    "publisherName".to_owned(),
                    JsonValue::String(product.publisher_name().to_owned()),
                ),
            ]);
            if let Some(name) = &self.start_menu_name {
                product_fields.insert(
                    "startMenuName".to_owned(),
                    JsonValue::String(name.as_str().to_owned()),
                );
            }
            fields.insert("product".to_owned(), JsonValue::Object(product_fields));
        }
        JsonValue::Object(fields).to_json()
    }

    fn format_minor(&self) -> u8 {
        if self.start_menu_name.is_some() {
            3
        } else if self.product_metadata.is_some() {
            2
        } else if self.update_catalogue.is_some() {
            1
        } else {
            0
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlanFormatVersion {
    Base,
    Catalogue,
    ProductMetadata,
    ProductRegistration,
}

impl PlanFormatVersion {
    const fn has_update_catalogue(self) -> bool {
        !matches!(self, Self::Base)
    }

    const fn has_product_metadata(self) -> bool {
        matches!(self, Self::ProductMetadata | Self::ProductRegistration)
    }

    const fn has_start_menu_name(self) -> bool {
        matches!(self, Self::ProductRegistration)
    }
}

fn parse_format_version(
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
        _ => Err(ReleaseManifestAuthorError::PlanInvalid),
    }
}

fn parse_product_metadata(
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

fn parse_capabilities(
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

fn parse_network_origins(
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

fn parse_update_catalogue(
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

fn exact_fields(
    fields: &BTreeMap<String, JsonValue>,
    expected: &[&str],
) -> Result<(), ReleaseManifestAuthorError> {
    (fields.len() == expected.len() && expected.iter().all(|name| fields.contains_key(*name)))
        .then_some(())
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}

fn required_object<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, ReleaseManifestAuthorError> {
    fields
        .get(name)
        .and_then(JsonValue::as_object)
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}

fn required_string<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, ReleaseManifestAuthorError> {
    fields
        .get(name)
        .and_then(JsonValue::as_string)
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}

fn required_u16(
    fields: &BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<u16, ReleaseManifestAuthorError> {
    fields
        .get(name)
        .and_then(JsonValue::as_u16)
        .ok_or(ReleaseManifestAuthorError::PlanInvalid)
}
