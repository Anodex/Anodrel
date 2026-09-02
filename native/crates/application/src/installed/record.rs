//! Strict parsing of the host-selected installed-application record.

use anodrel_json::JsonValue;
use anodrel_network::{NetworkOrigin, NetworkOriginPolicy};
use anodrel_protocol::Capability;

use crate::{MAX_INSTALL_RECORD_BYTES, manifest, network_policy, sha256};

use super::{
    InstalledApplicationError, ProductDisplayMetadata, UpdateCatalogueLocation, exact_fields,
    is_valid_executable_path, required_object, required_string, validate_version,
};

pub(super) struct ParsedRecord {
    pub(super) application_id: String,
    pub(super) package_root: String,
    pub(super) executable_path: String,
    pub(super) executable_digest: [u8; 32],
    pub(super) publisher_fingerprint: [u8; 32],
    pub(super) capabilities: Vec<Capability>,
    pub(super) network_policy: Option<NetworkOriginPolicy>,
    pub(super) update_catalogue: Option<UpdateCatalogueLocation>,
    pub(super) product_metadata: Option<ProductDisplayMetadata>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RecordVersion {
    V1_0,
    V1_1,
    V1_2,
    V1_3,
    V1_4,
    V1_5,
    V1_6,
    V1_7,
    V1_8,
    V1_9,
    V1_10,
    V1_11,
    V1_12,
    V1_13,
    V1_14,
    V1_15,
    V1_16,
    V1_17,
    V1_18,
    V1_19,
    V1_20,
    V1_21,
}

impl RecordVersion {
    /// Returns whether this strictly ordered record version includes grants
    /// introduced by `minimum`. Keeping the comparison here prevents a newer
    /// record from accidentally losing an older optional grant when another
    /// version is added.
    const fn accepts(self, minimum: Self) -> bool {
        (self as u8) >= (minimum as u8)
    }

    /// Whether this version accepts the grants version 1.2 introduced.
    ///
    /// Each later version is a superset, so a record written for 1.2 keeps its
    /// exact meaning when read as 1.3 or 1.4.
    const fn accepts_v1_2_grants(self) -> bool {
        self.accepts(Self::V1_2)
    }

    /// Whether this version accepts the grant version 1.3 introduced.
    const fn accepts_v1_3_grants(self) -> bool {
        self.accepts(Self::V1_3)
    }

    /// Whether this version accepts the grant version 1.4 introduced.
    const fn accepts_v1_4_grants(self) -> bool {
        self.accepts(Self::V1_4)
    }

    /// Whether this version accepts the grant version 1.5 introduced.
    const fn accepts_v1_5_grants(self) -> bool {
        self.accepts(Self::V1_5)
    }
}

pub(super) fn parse(input: &str) -> Result<ParsedRecord, InstalledApplicationError> {
    if input.len() > MAX_INSTALL_RECORD_BYTES {
        return Err(InstalledApplicationError::RecordTooLarge);
    }
    let root = JsonValue::parse(input).map_err(|_| InstalledApplicationError::InvalidRecord)?;
    let fields = root
        .as_object()
        .ok_or(InstalledApplicationError::InvalidRecord)?;
    let version = validate_version(required_object(fields, "recordVersion")?)?;
    let expected_fields = if version == RecordVersion::V1_0 {
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
        ][..]
    } else if version.accepts(RecordVersion::V1_21) {
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
            "capabilities",
            "networkOrigins",
            "updateCatalogue",
            "product",
        ][..]
    } else if version.accepts(RecordVersion::V1_20) {
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
            "capabilities",
            "networkOrigins",
            "updateCatalogue",
        ][..]
    } else if version.accepts(RecordVersion::V1_14) {
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
            "capabilities",
            "networkOrigins",
        ][..]
    } else {
        &[
            "recordVersion",
            "applicationId",
            "packageRoot",
            "executable",
            "publisher",
            "capabilities",
        ][..]
    };
    exact_fields(fields, expected_fields)?;

    let application_id = required_string(fields, "applicationId")?;
    if !manifest::is_valid_application_id(application_id) {
        return Err(InstalledApplicationError::InvalidRecord);
    }
    let package_root = required_string(fields, "packageRoot")?;
    if package_root.is_empty() || package_root.chars().any(char::is_control) {
        return Err(InstalledApplicationError::InvalidRecord);
    }

    let executable = required_object(fields, "executable")?;
    exact_fields(executable, &["path", "sha256"])?;
    let executable_path = required_string(executable, "path")?;
    if !is_valid_executable_path(executable_path) {
        return Err(InstalledApplicationError::InvalidExecutablePath);
    }
    let executable_digest = sha256::parse_lower_hex(required_string(executable, "sha256")?)
        .ok_or(InstalledApplicationError::InvalidRecord)?;

    let publisher = required_object(fields, "publisher")?;
    exact_fields(publisher, &["leafCertificateSha256"])?;
    let publisher_fingerprint =
        sha256::parse_lower_hex(required_string(publisher, "leafCertificateSha256")?)
            .ok_or(InstalledApplicationError::InvalidRecord)?;

    let capabilities = if version == RecordVersion::V1_0 {
        Vec::new()
    } else {
        let Some(JsonValue::Array(values)) = fields.get("capabilities") else {
            return Err(InstalledApplicationError::InvalidRecord);
        };
        let mut grants = Vec::with_capacity(values.len());
        for value in values {
            let capability = capability_for_record_version(version, value.as_string())
                .ok_or(InstalledApplicationError::InvalidRecord)?;
            if grants.contains(&capability) {
                return Err(InstalledApplicationError::InvalidRecord);
            }
            grants.push(capability);
        }
        grants
    };
    let network_policy = if version.accepts(RecordVersion::V1_14) {
        network_policy::parse_network_policy(
            fields,
            capabilities.contains(&Capability::NetworkFetch),
        )?
    } else {
        None
    };
    let update_catalogue = if version.accepts(RecordVersion::V1_20) {
        Some(parse_update_catalogue(required_object(
            fields,
            "updateCatalogue",
        )?)?)
    } else {
        None
    };
    let product_metadata = if version.accepts(RecordVersion::V1_21) {
        Some(parse_product_metadata(required_object(fields, "product")?)?)
    } else {
        None
    };

    Ok(ParsedRecord {
        application_id: application_id.to_owned(),
        package_root: package_root.to_owned(),
        executable_path: executable_path.to_owned(),
        executable_digest,
        publisher_fingerprint,
        capabilities,
        network_policy,
        update_catalogue,
        product_metadata,
    })
}

fn parse_product_metadata(
    fields: &std::collections::BTreeMap<String, JsonValue>,
) -> Result<ProductDisplayMetadata, InstalledApplicationError> {
    exact_fields(fields, &["displayName", "publisherName"])?;
    ProductDisplayMetadata::new(
        required_string(fields, "displayName")?,
        required_string(fields, "publisherName")?,
    )
    .map_err(|_| InstalledApplicationError::InvalidRecord)
}

fn parse_update_catalogue(
    fields: &std::collections::BTreeMap<String, JsonValue>,
) -> Result<UpdateCatalogueLocation, InstalledApplicationError> {
    exact_fields(fields, &["origin", "path"])?;
    let origin = required_object(fields, "origin")?;
    exact_fields(origin, &["host", "port"])?;
    let origin = NetworkOrigin::new(
        required_string(origin, "host")?,
        origin
            .get("port")
            .and_then(JsonValue::as_u16)
            .ok_or(InstalledApplicationError::InvalidRecord)?,
    )
    .map_err(|_| InstalledApplicationError::InvalidRecord)?;
    UpdateCatalogueLocation::new(origin, required_string(fields, "path")?)
        .map_err(|_| InstalledApplicationError::InvalidRecord)
}

fn capability_for_record_version(
    version: RecordVersion,
    value: Option<&str>,
) -> Option<Capability> {
    match value? {
        "diagnostics.read" => Some(Capability::DiagnosticsRead),
        "ui.document.write" => Some(Capability::UiDocumentWrite),
        "ui.events.read" => Some(Capability::UiEventsRead),
        "session.close" => Some(Capability::SessionClose),
        "clipboard.read" => Some(Capability::ClipboardRead),
        "clipboard.write" => Some(Capability::ClipboardWrite),
        "external.open" => Some(Capability::ExternalOpen),
        "dialog.open_file" if version.accepts_v1_2_grants() => Some(Capability::DialogOpenFile),
        "dialog.save_file" if version.accepts_v1_2_grants() => Some(Capability::DialogSaveFile),
        "file.read_text" if version.accepts_v1_2_grants() => Some(Capability::FileReadText),
        "storage.state.read" if version.accepts_v1_2_grants() => Some(Capability::StorageStateRead),
        "storage.state.replace" if version.accepts_v1_2_grants() => {
            Some(Capability::StorageStateReplace)
        }
        "storage.state.clear" if version.accepts_v1_2_grants() => {
            Some(Capability::StorageStateClear)
        }
        "credential.read" if version.accepts_v1_2_grants() => Some(Capability::CredentialRead),
        "credential.write" if version.accepts_v1_2_grants() => Some(Capability::CredentialWrite),
        "credential.delete" if version.accepts_v1_2_grants() => Some(Capability::CredentialDelete),
        // Each later record version adds named grants deliberately. An earlier
        // record naming a later grant stays invalid, so provisioning cannot
        // widen a record by accident.
        "notification.show" if version.accepts_v1_3_grants() => Some(Capability::NotificationShow),
        "window.title" if version.accepts_v1_4_grants() => Some(Capability::WindowTitle),
        "ui.fields.read" if version.accepts_v1_5_grants() => Some(Capability::UiFieldsRead),
        "window.state" if version.accepts(RecordVersion::V1_6) => Some(Capability::WindowState),
        "file.write_text" if version.accepts(RecordVersion::V1_7) => {
            Some(Capability::FileWriteText)
        }
        "menu.write" if version.accepts(RecordVersion::V1_8) => Some(Capability::MenuWrite),
        "window.focus" if version.accepts(RecordVersion::V1_9) => Some(Capability::WindowFocus),
        "window.fullscreen" if version.accepts(RecordVersion::V1_10) => {
            Some(Capability::WindowFullscreen)
        }
        "file.write_binary" if version.accepts(RecordVersion::V1_11) => {
            Some(Capability::FileWriteBinary)
        }
        "window.size" if version.accepts(RecordVersion::V1_12) => Some(Capability::WindowSize),
        "window.open" if version.accepts(RecordVersion::V1_13) => Some(Capability::WindowOpen),
        "window.close" if version.accepts(RecordVersion::V1_13) => Some(Capability::WindowClose),
        "network.fetch" if version.accepts(RecordVersion::V1_14) => Some(Capability::NetworkFetch),
        "dialog.open_folder" if version.accepts(RecordVersion::V1_15) => {
            Some(Capability::DialogOpenFolder)
        }
        "folder.read_entries" if version.accepts(RecordVersion::V1_16) => {
            Some(Capability::FolderReadEntries)
        }
        "window.state.read" if version.accepts(RecordVersion::V1_17) => {
            Some(Capability::WindowStateRead)
        }
        "window.state.observe" if version.accepts(RecordVersion::V1_18) => {
            Some(Capability::WindowStateObserve)
        }
        "menu.context.write" if version.accepts(RecordVersion::V1_19) => {
            Some(Capability::ContextMenuWrite)
        }
        _ => None,
    }
}
