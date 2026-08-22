//! Installed-record network-origin policy parsing.
//!
//! The record remains machine-selected. This module turns its one strictly
//! shaped origin array into the portable exact-origin value that a native host
//! can later compose into a network service.

use std::collections::BTreeMap;

use anodrel_json::JsonValue;
use anodrel_network::{NetworkOrigin, NetworkOriginPolicy};

use super::{
    InstalledApplicationError,
    installed::{exact_fields, required_string},
};

/// Parses version 1.14's required `networkOrigins` field.
///
/// A grant and a non-empty policy are inseparable: accepting either one alone
/// would create latent or ambient network authority in an installed record.
pub(super) fn parse_network_policy(
    fields: &BTreeMap<String, JsonValue>,
    network_granted: bool,
) -> Result<Option<NetworkOriginPolicy>, InstalledApplicationError> {
    let Some(JsonValue::Array(values)) = fields.get("networkOrigins") else {
        return Err(InstalledApplicationError::InvalidRecord);
    };

    if !network_granted {
        return values
            .is_empty()
            .then_some(None)
            .ok_or(InstalledApplicationError::InvalidRecord);
    }

    let mut origins = Vec::with_capacity(values.len());
    for value in values {
        let fields = value
            .as_object()
            .ok_or(InstalledApplicationError::InvalidRecord)?;
        exact_fields(fields, &["host", "port"])?;
        let host = required_string(fields, "host")?;
        let port = fields
            .get("port")
            .and_then(JsonValue::as_u16)
            .ok_or(InstalledApplicationError::InvalidRecord)?;
        let origin =
            NetworkOrigin::new(host, port).map_err(|_| InstalledApplicationError::InvalidRecord)?;
        origins.push(origin);
    }

    NetworkOriginPolicy::new(origins)
        .map(Some)
        .map_err(|_| InstalledApplicationError::InvalidRecord)
}
