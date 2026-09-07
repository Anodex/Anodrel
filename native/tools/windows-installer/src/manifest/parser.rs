//! Parsing of the strict signed `anodrel.release.v1` release facts.

use std::collections::BTreeMap;

use anodrel_application::{UpdateCatalogueLocation, is_valid_application_id, sha256};
use anodrel_json::JsonValue;
use anodrel_network::{NetworkOrigin, NetworkOriginPolicy};
use anodrel_protocol::Capability;

use crate::{MAX_PAYLOAD_BYTES, MAX_RELEASE_MANIFEST_BYTES, ReleaseManifestError};

use super::fields::{
    exact_fields, is_valid_executable_path, required_object, required_string, required_u16,
    required_u64,
};
use super::{PackageVersion, PayloadDescriptor, ProductLauncher, ReleaseManifest, product};

impl ReleaseManifest {
    /// Parses one strict version-1 `anodrel.release.v1` manifest.
    pub fn parse(input: &str) -> Result<Self, ReleaseManifestError> {
        if input.len() > MAX_RELEASE_MANIFEST_BYTES {
            return Err(ReleaseManifestError::TooLarge);
        }
        let root = JsonValue::parse(input).map_err(|_| ReleaseManifestError::Invalid)?;
        let fields = root.as_object().ok_or(ReleaseManifestError::Invalid)?;
        let format_version = super::format::parse(required_object(fields, "formatVersion")?)?;
        let expected_fields = match format_version {
            super::format::FormatVersion::Base => &[
                "formatVersion",
                "applicationId",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "payload",
            ][..],
            super::format::FormatVersion::Catalogue => &[
                "formatVersion",
                "applicationId",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "updateCatalogue",
                "payload",
            ][..],
            super::format::FormatVersion::ProductMetadata => &[
                "formatVersion",
                "applicationId",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "updateCatalogue",
                "product",
                "payload",
            ][..],
            super::format::FormatVersion::ProductRegistration => &[
                "formatVersion",
                "applicationId",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "updateCatalogue",
                "product",
                "payload",
            ][..],
            super::format::FormatVersion::ProductLauncher => &[
                "formatVersion",
                "applicationId",
                "packageVersion",
                "executable",
                "publisher",
                "capabilities",
                "networkOrigins",
                "updateCatalogue",
                "product",
                "launcher",
                "payload",
            ][..],
        };
        exact_fields(fields, expected_fields)?;

        let application_id = required_string(fields, "applicationId")?;
        if !is_valid_application_id(application_id) {
            return Err(ReleaseManifestError::Invalid);
        }

        let package_version = parse_package_version(required_object(fields, "packageVersion")?)?;
        let (executable_path, executable_digest) =
            parse_executable(required_object(fields, "executable")?)?;
        let publisher_fingerprint = parse_publisher(required_object(fields, "publisher")?)?;
        let capabilities = parse_capabilities(fields.get("capabilities"))?;
        let network_origins = parse_network_origins(fields.get("networkOrigins"), &capabilities)?;
        let update_catalogue = format_version
            .has_update_catalogue()
            .then(|| parse_update_catalogue(required_object(fields, "updateCatalogue")?))
            .transpose()?;
        let (product_metadata, start_menu_name) = if format_version.has_product_metadata() {
            let (metadata, name) = product::parse(
                required_object(fields, "product")?,
                format_version.has_start_menu_name(),
            )?;
            (Some(metadata), name)
        } else {
            (None, None)
        };
        let product_launcher = format_version
            .has_product_launcher()
            .then(|| ProductLauncher::parse(required_object(fields, "launcher")?))
            .transpose()?;
        if product_launcher
            .as_ref()
            .is_some_and(|launcher| launcher.path().eq_ignore_ascii_case(&executable_path))
        {
            return Err(ReleaseManifestError::ExecutablePathInvalid);
        }
        let payload = parse_payload(required_object(fields, "payload")?)?;

        Ok(Self {
            application_id: application_id.to_owned(),
            package_version,
            executable_path,
            executable_digest,
            publisher_fingerprint,
            capabilities,
            network_origins,
            update_catalogue,
            product_metadata,
            start_menu_name,
            product_launcher,
            payload,
        })
    }
}

fn parse_update_catalogue(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<UpdateCatalogueLocation, ReleaseManifestError> {
    exact_fields(fields, &["origin", "path"])?;
    let origin = required_object(fields, "origin")?;
    exact_fields(origin, &["host", "port"])?;
    let origin = NetworkOrigin::new(
        required_string(origin, "host")?,
        required_u16(origin, "port")?,
    )
    .map_err(|_| ReleaseManifestError::PolicyInvalid)?;
    UpdateCatalogueLocation::new(origin, required_string(fields, "path")?)
        .map_err(|_| ReleaseManifestError::PolicyInvalid)
}

fn parse_package_version(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<PackageVersion, ReleaseManifestError> {
    exact_fields(fields, &["major", "minor", "patch"])?;
    Ok(PackageVersion::new(
        required_u16(fields, "major")?,
        required_u16(fields, "minor")?,
        required_u16(fields, "patch")?,
    ))
}

fn parse_executable(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<(String, [u8; 32]), ReleaseManifestError> {
    exact_fields(fields, &["path", "sha256"])?;
    let path = required_string(fields, "path")?;
    if !is_valid_executable_path(path) {
        return Err(ReleaseManifestError::ExecutablePathInvalid);
    }
    let digest = sha256::parse_lower_hex(required_string(fields, "sha256")?)
        .ok_or(ReleaseManifestError::Invalid)?;
    Ok((path.to_owned(), digest))
}

fn parse_publisher(fields: &BTreeMap<String, JsonValue>) -> Result<[u8; 32], ReleaseManifestError> {
    exact_fields(fields, &["leafCertificateSha256"])?;
    sha256::parse_lower_hex(required_string(fields, "leafCertificateSha256")?)
        .ok_or(ReleaseManifestError::Invalid)
}

fn parse_capabilities(value: Option<&JsonValue>) -> Result<Vec<Capability>, ReleaseManifestError> {
    let Some(JsonValue::Array(values)) = value else {
        return Err(ReleaseManifestError::PolicyInvalid);
    };
    let mut capabilities = Vec::with_capacity(values.len());
    for value in values {
        let capability =
            capability_from_name(value.as_string()).ok_or(ReleaseManifestError::PolicyInvalid)?;
        if capabilities.contains(&capability) {
            return Err(ReleaseManifestError::PolicyInvalid);
        }
        capabilities.push(capability);
    }
    Ok(capabilities)
}

fn parse_network_origins(
    value: Option<&JsonValue>,
    capabilities: &[Capability],
) -> Result<Vec<NetworkOrigin>, ReleaseManifestError> {
    let Some(JsonValue::Array(values)) = value else {
        return Err(ReleaseManifestError::PolicyInvalid);
    };
    let mut origins = Vec::with_capacity(values.len());
    for value in values {
        let fields = value
            .as_object()
            .ok_or(ReleaseManifestError::PolicyInvalid)?;
        exact_fields(fields, &["host", "port"])?;
        let origin = NetworkOrigin::new(
            required_string(fields, "host")?,
            required_u16(fields, "port")?,
        )
        .map_err(|_| ReleaseManifestError::PolicyInvalid)?;
        origins.push(origin);
    }

    if capabilities.contains(&Capability::NetworkFetch) {
        NetworkOriginPolicy::new(origins.clone())
            .map_err(|_| ReleaseManifestError::PolicyInvalid)?;
    } else if !origins.is_empty() {
        return Err(ReleaseManifestError::PolicyInvalid);
    }
    Ok(origins)
}

fn parse_payload(
    fields: &BTreeMap<String, JsonValue>,
) -> Result<PayloadDescriptor, ReleaseManifestError> {
    exact_fields(fields, &["byteLength", "sha256"])?;
    let byte_length = required_u64(fields, "byteLength")?;
    if !(1..=MAX_PAYLOAD_BYTES).contains(&byte_length) {
        return Err(ReleaseManifestError::PayloadInvalid);
    }
    let digest = sha256::parse_lower_hex(required_string(fields, "sha256")?)
        .ok_or(ReleaseManifestError::PayloadInvalid)?;
    Ok(PayloadDescriptor {
        byte_length,
        digest,
    })
}

fn capability_from_name(value: Option<&str>) -> Option<Capability> {
    match value? {
        "diagnostics.read" => Some(Capability::DiagnosticsRead),
        "ui.document.write" => Some(Capability::UiDocumentWrite),
        "ui.events.read" => Some(Capability::UiEventsRead),
        "session.close" => Some(Capability::SessionClose),
        "clipboard.read" => Some(Capability::ClipboardRead),
        "clipboard.write" => Some(Capability::ClipboardWrite),
        "external.open" => Some(Capability::ExternalOpen),
        "network.fetch" => Some(Capability::NetworkFetch),
        "dialog.open_file" => Some(Capability::DialogOpenFile),
        "dialog.open_folder" => Some(Capability::DialogOpenFolder),
        "folder.read_entries" => Some(Capability::FolderReadEntries),
        "dialog.save_file" => Some(Capability::DialogSaveFile),
        "file.read_text" => Some(Capability::FileReadText),
        "file.write_text" => Some(Capability::FileWriteText),
        "file.write_binary" => Some(Capability::FileWriteBinary),
        "storage.state.read" => Some(Capability::StorageStateRead),
        "storage.state.replace" => Some(Capability::StorageStateReplace),
        "storage.state.clear" => Some(Capability::StorageStateClear),
        "credential.read" => Some(Capability::CredentialRead),
        "credential.write" => Some(Capability::CredentialWrite),
        "credential.delete" => Some(Capability::CredentialDelete),
        "notification.show" => Some(Capability::NotificationShow),
        "window.title" => Some(Capability::WindowTitle),
        "window.state" => Some(Capability::WindowState),
        "window.state.read" => Some(Capability::WindowStateRead),
        "window.state.observe" => Some(Capability::WindowStateObserve),
        "window.focus" => Some(Capability::WindowFocus),
        "window.fullscreen" => Some(Capability::WindowFullscreen),
        "window.size" => Some(Capability::WindowSize),
        "window.open" => Some(Capability::WindowOpen),
        "window.close" => Some(Capability::WindowClose),
        "ui.fields.read" => Some(Capability::UiFieldsRead),
        "menu.write" => Some(Capability::MenuWrite),
        "menu.context.write" => Some(Capability::ContextMenuWrite),
        _ => None,
    }
}
