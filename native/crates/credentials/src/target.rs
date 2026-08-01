use std::fmt;

use anodrel_application::{ApplicationIdentity, is_valid_application_id};

use crate::{CredentialInputError, CredentialName};

/// An exact generic-credential target derived from trusted host identity.
#[derive(Clone, Eq, PartialEq)]
pub struct CredentialTarget(String);

impl CredentialTarget {
    /// Derives the fixed v1 target namespace without credential-store I/O.
    pub fn for_application(
        identity: &ApplicationIdentity,
        name: &CredentialName,
    ) -> Result<Self, CredentialInputError> {
        if !is_valid_application_id(identity.application_id()) {
            return Err(CredentialInputError::InvalidApplicationIdentity);
        }
        Ok(Self(format!(
            "Anodrel/v1/{}/{}",
            identity.application_id(),
            name.as_str()
        )))
    }

    /// Returns the exact native-store target for immediate adapter use.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CredentialTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CredentialTarget(..)")
    }
}

#[cfg(test)]
mod tests {
    use anodrel_application::ApplicationManifest;

    use crate::{CredentialName, CredentialTarget};

    fn identity() -> anodrel_application::ApplicationIdentity {
        ApplicationManifest::parse(
            r#"{
                "manifestVersion": { "major": 1, "minor": 0 },
                "applicationId": "org.anodrel.credentials-test",
                "displayName": "Credentials Test",
                "content": {
                    "format": "anodrel.text.v1",
                    "path": "content/main.txt",
                    "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                }
            }"#,
        )
        .expect("fixture manifest is valid")
        .identity()
        .clone()
    }

    #[test]
    fn derives_one_exact_target_from_identity_and_name() {
        let name = CredentialName::parse("refresh-token").expect("fixture name is valid");
        let target = CredentialTarget::for_application(&identity(), &name)
            .expect("fixture identity is valid");
        assert_eq!(
            target.as_str(),
            "Anodrel/v1/org.anodrel.credentials-test/refresh-token"
        );
    }

    #[test]
    fn debug_output_redacts_the_target() {
        let name = CredentialName::parse("refresh-token").expect("fixture name is valid");
        let target = CredentialTarget::for_application(&identity(), &name)
            .expect("fixture identity is valid");
        assert_eq!(format!("{target:?}"), "CredentialTarget(..)");
    }
}
