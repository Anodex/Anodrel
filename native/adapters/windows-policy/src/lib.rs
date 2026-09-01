#![deny(unsafe_op_in_unsafe_fn)]

//! Read-only machine policy for installed Windows applications.
//!
//! This adapter reads the fixed current `record` and, for installer recovery
//! only, the fixed retained `previous` `REG_SZ` record from the 64-bit
//! `HKEY_LOCAL_MACHINE` Anodrel policy location. It delegates every package and
//! executable check to `anodrel-application`. It does not create, modify,
//! delete, enumerate, or otherwise provision registry policy. It does not
//! verify Authenticode, launch a process, create a pipe, or expose policy data
//! to an application or renderer.

mod raw;

use std::fmt;

use anodrel_application::{
    InstalledApplication, InstalledApplicationError, is_valid_application_id,
};
use anodrel_core::HostPolicy;
use anodrel_session_policy::{SessionPolicyError, host_policy_for_installed_application};

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

/// Loads the fixed retained prior record for an installer-only rollback preflight.
///
/// This is not host session policy. `application_id` must be selected by a
/// signed installer release rather than an application, package, protocol
/// message, environment value, command line, or UI control.
pub fn load_previous_installed_application(
    application_id: &str,
) -> Result<InstalledApplication, PolicyStoreError> {
    if !is_valid_application_id(application_id) {
        return Err(PolicyStoreError::InvalidApplicationId);
    }
    let record = raw::read_previous_record(application_id)?;
    InstalledApplication::load_from_trusted_record(&record, application_id)
        .map_err(PolicyStoreError::Record)
}

/// Loads machine policy and creates the policy for one authenticated host session.
///
/// Both the installed record and its capability grants are selected only from
/// the fixed machine store. The caller supplies a local host label; it is not
/// an application-facing value and is used only in protocol diagnostics.
/// This does not create a pipe, launch a process, or expose the policy to an
/// application surface.
pub fn load_host_policy(
    application_id: &str,
    host_name: impl Into<String>,
) -> Result<HostPolicy, PolicyStoreError> {
    let application = load_installed_application(application_id)?;
    host_policy_for_installed_application(&application, host_name)
        .map_err(PolicyStoreError::SessionPolicy)
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
    SessionPolicy(SessionPolicyError),
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
            Self::SessionPolicy(_) => "installed application session policy could not be created",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PolicyStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Record(error) => Some(error),
            Self::SessionPolicy(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PolicyStoreError, load_host_policy, load_installed_application,
        load_previous_installed_application,
    };

    #[test]
    fn rejects_an_invalid_identity_before_opening_a_registry_key() {
        assert!(matches!(
            load_installed_application("org.anodrel/escape"),
            Err(PolicyStoreError::InvalidApplicationId)
        ));
    }

    #[test]
    fn rejects_an_invalid_session_identity_before_opening_a_registry_key() {
        assert!(matches!(
            load_host_policy("org.anodrel/escape", "windows-host"),
            Err(PolicyStoreError::InvalidApplicationId)
        ));
    }

    #[test]
    fn prior_policy_lookup_rejects_an_invalid_identity_before_opening_a_registry_key() {
        assert!(matches!(
            load_previous_installed_application("org.anodrel/escape"),
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
