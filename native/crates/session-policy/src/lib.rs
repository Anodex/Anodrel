#![forbid(unsafe_code)]

//! Converts validated installed-application policy into one host session policy.
//!
//! This crate is deliberately platform-neutral. An operating-system adapter
//! selects and validates an installed record first; this module only carries
//! that record's identity and fixed machine grants into `anodrel-core`.
//! Application packages, bootstrap data, pipe clients, protocol messages, and
//! UI cannot choose the resulting grants.

use std::fmt;

use anodrel_application::{ApplicationIdentity, InstalledApplication};
use anodrel_core::HostPolicy;
use anodrel_protocol::Capability;

/// Builds the policy used by one authenticated host session for an installed
/// application.
///
/// The application identity and grants are copied only from a record that
/// `anodrel-application` already validated. A version 1.0 installed record
/// supplies an empty grant list; a version 1.1 record supplies its validated
/// machine-policy grants, including a possible `ui.document.write` grant.
pub fn host_policy_for_installed_application(
    application: &InstalledApplication,
    host_name: impl Into<String>,
) -> Result<HostPolicy, SessionPolicyError> {
    host_policy_for_identity(
        application.identity(),
        application.capabilities(),
        host_name,
    )
}

fn host_policy_for_identity(
    identity: &ApplicationIdentity,
    capabilities: &[Capability],
    host_name: impl Into<String>,
) -> Result<HostPolicy, SessionPolicyError> {
    HostPolicy::new(identity.application_id(), capabilities.to_vec(), host_name)
        .map_err(|_| SessionPolicyError::InvalidHostName)
}

/// A failure while converting trusted installed policy into a session policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPolicyError {
    /// A host did not supply a valid local host name.
    InvalidHostName,
}

impl fmt::Display for SessionPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("host session policy could not be created")
    }
}

impl std::error::Error for SessionPolicyError {}

#[cfg(test)]
mod tests {
    use anodrel_application::ApplicationManifest;
    use anodrel_core::CoreHost;
    use anodrel_protocol::{Capability, JsonValue};

    use super::{SessionPolicyError, host_policy_for_identity};

    const MANIFEST: &str = r#"{
        "manifestVersion":{"major":1,"minor":0},
        "applicationId":"org.anodrel.sample",
        "displayName":"Anodrel Sample",
        "content":{
            "format":"anodrel.text.v1",
            "path":"content.txt",
            "sha256":"0000000000000000000000000000000000000000000000000000000000000000"
        }
    }"#;

    #[test]
    fn maps_validated_machine_grants_to_the_host_session() {
        let manifest = ApplicationManifest::parse(MANIFEST).expect("fixture manifest is valid");
        let policy = host_policy_for_identity(
            manifest.identity(),
            &[Capability::DiagnosticsRead],
            "windows-host",
        )
        .expect("non-empty host name is accepted");

        let response = CoreHost::new(policy).handle_json(
            r#"{
                "protocolVersion":{"major":1,"minor":0},
                "kind":"request",
                "requestId":"capabilities",
                "operation":"platform.capabilities",
                "payload":{}
            }"#,
        );
        let response = JsonValue::parse(&response).expect("response is valid JSON");
        let result = response
            .as_object()
            .and_then(|fields| fields.get("result"))
            .and_then(JsonValue::as_object)
            .expect("capability response has a result");

        assert_eq!(
            result.get("applicationId").and_then(JsonValue::as_string),
            Some("org.anodrel.sample")
        );
        assert_eq!(
            result.get("grantedCapabilities"),
            Some(&JsonValue::Array(vec![JsonValue::String(
                "diagnostics.read".to_owned()
            )]))
        );
    }

    #[test]
    fn rejects_an_empty_host_name() {
        let manifest = ApplicationManifest::parse(MANIFEST).expect("fixture manifest is valid");

        assert!(matches!(
            host_policy_for_identity(manifest.identity(), &[], " "),
            Err(SessionPolicyError::InvalidHostName)
        ));
    }
}
