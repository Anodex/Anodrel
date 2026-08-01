use std::fmt;

use crate::{CredentialInputError, MAX_CREDENTIAL_NAME_BYTES};

/// A restricted host-selected credential name.
#[derive(Clone, Eq, PartialEq)]
pub struct CredentialName(String);

impl CredentialName {
    /// Parses a v1 credential name without filesystem or credential-store I/O.
    pub fn parse(value: &str) -> Result<Self, CredentialInputError> {
        let bytes = value.as_bytes();
        if !(1..=MAX_CREDENTIAL_NAME_BYTES).contains(&bytes.len())
            || !matches!(bytes.first(), Some(b'a'..=b'z' | b'0'..=b'9'))
            || !matches!(bytes.last(), Some(b'a'..=b'z' | b'0'..=b'9'))
            || !bytes
                .iter()
                .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_'))
        {
            return Err(CredentialInputError::InvalidCredentialName);
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the validated credential-name component for a native adapter.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CredentialName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CredentialName(..)")
    }
}

#[cfg(test)]
mod tests {
    use crate::{CredentialInputError, CredentialName, MAX_CREDENTIAL_NAME_BYTES};

    #[test]
    fn accepts_the_restricted_v1_grammar() {
        let name =
            CredentialName::parse("refresh_token-2.internal").expect("restricted name is valid");
        assert_eq!(name.as_str(), "refresh_token-2.internal");
    }

    #[test]
    fn rejects_target_separators_controls_and_bad_bounds() {
        for value in [
            "",
            "/other-app",
            "scope/name",
            "name\\value",
            "credential:name",
            "name\nvalue",
            "-starts-with-symbol",
            "ends-with-symbol_",
            &"a".repeat(MAX_CREDENTIAL_NAME_BYTES + 1),
        ] {
            assert_eq!(
                CredentialName::parse(value),
                Err(CredentialInputError::InvalidCredentialName)
            );
        }
    }

    #[test]
    fn debug_output_redacts_the_credential_name() {
        let name = CredentialName::parse("refresh-token").expect("fixture is valid");
        assert_eq!(format!("{name:?}"), "CredentialName(..)");
    }
}
