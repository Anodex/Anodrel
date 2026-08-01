#![deny(unsafe_op_in_unsafe_fn)]

//! Read-only machine policy for installed Windows applications.
//!
//! This adapter reads exactly one `REG_SZ` record from the fixed 64-bit
//! `HKEY_LOCAL_MACHINE` Anodrel policy location, then delegates every package
//! and executable check to `anodrel-application`. It does not create, modify,
//! delete, enumerate, or otherwise provision registry policy. It does not
//! verify Authenticode, launch a process, create a pipe, or expose policy data
//! to an application or renderer.

mod raw;

use std::fmt;

use anodrel_application::{
    InstalledApplication, InstalledApplicationError, is_valid_application_id,
};

/// Loads one installed application record from the fixed machine policy store.
///
/// `application_id` must be selected by host policy rather than an application,
/// package, protocol message, environment value, command line, or UI control.
pub fn load_installed_application(
    application_id: &str,
) -> Result<InstalledApplication, PolicyStoreError> {
    if !is_valid_application_id(application_id) {
        return Err(PolicyStoreError::InvalidApplicationId);
    }
    let record = raw::read_record(application_id)?;
    InstalledApplication::load_from_trusted_record(&record, application_id)
        .map_err(PolicyStoreError::Record)
}

/// A safe failure category while reading machine policy.
#[derive(Debug)]
pub enum PolicyStoreError {
    InvalidApplicationId,
    RecordNotFound,
    AccessDenied,
    RecordNotString,
    RecordTooLarge,
    RecordMalformed,
    RecordChangedDuringRead,
    RegistryUnavailable,
    Record(InstalledApplicationError),
}

impl fmt::Display for PolicyStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidApplicationId => "installed application identity is invalid",
            Self::RecordNotFound => "installed application policy record was not found",
            Self::AccessDenied => "installed application policy record cannot be read",
            Self::RecordNotString => "installed application policy record has an invalid type",
            Self::RecordTooLarge => "installed application policy record exceeds its limit",
            Self::RecordMalformed => "installed application policy record is malformed",
            Self::RecordChangedDuringRead => {
                "installed application policy record changed while being read"
            }
            Self::RegistryUnavailable => "Windows machine policy is unavailable",
            Self::Record(_) => "installed application policy record did not validate",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PolicyStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Record(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PolicyStoreError, load_installed_application};

    #[test]
    fn rejects_an_invalid_identity_before_opening_a_registry_key() {
        assert!(matches!(
            load_installed_application("org.anodrel/escape"),
            Err(PolicyStoreError::InvalidApplicationId)
        ));
    }

    #[test]
    fn an_unprovisioned_machine_record_is_a_safe_not_found_result() {
        assert!(matches!(
            load_installed_application("org.anodrel.policy-reader-unprovisioned-test"),
            Err(PolicyStoreError::RecordNotFound)
        ));
    }
}
