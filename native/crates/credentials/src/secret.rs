use std::fmt;

use crate::{CredentialInputError, MAX_SECRET_BYTES};

/// An opaque bounded credential value held only by trusted native host code.
pub struct Secret {
    bytes: Vec<u8>,
}

impl Secret {
    /// Creates a non-empty v1 secret without persistence or logging.
    pub fn new(bytes: Vec<u8>) -> Result<Self, CredentialInputError> {
        if bytes.is_empty() {
            return Err(CredentialInputError::EmptySecret);
        }
        if bytes.len() > MAX_SECRET_BYTES {
            return Err(CredentialInputError::SecretTooLarge);
        }
        Ok(Self { bytes })
    }

    /// Borrows secret bytes for an immediate trusted native-store operation.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Encodes this opaque secret as canonical lowercase hexadecimal.
    ///
    /// The returned value is intended only for an explicit authenticated
    /// protocol boundary. Callers must still treat it as secret material and
    /// must not log, render, persist, or place it in process arguments.
    #[must_use]
    pub fn to_lower_hex(&self) -> String {
        let mut encoded = String::with_capacity(self.bytes.len() * 2);
        for byte in &self.bytes {
            use std::fmt::Write as _;
            let _ = write!(encoded, "{byte:02x}");
        }
        encoded
    }

    /// Decodes one exact lowercase hexadecimal secret value.
    ///
    /// Hex preserves arbitrary bytes without treating a secret as UTF-8. It
    /// accepts neither whitespace nor alternate uppercase spellings, keeping
    /// the future protocol representation canonical.
    pub fn from_lower_hex(value: &str) -> Result<Self, CredentialInputError> {
        if value.is_empty() {
            return Err(CredentialInputError::EmptySecret);
        }
        if !value.len().is_multiple_of(2) || value.len() > MAX_SECRET_BYTES * 2 {
            return Err(CredentialInputError::InvalidSecretEncoding);
        }
        let mut bytes = Vec::with_capacity(value.len() / 2);
        for pair in value.as_bytes().chunks_exact(2) {
            let Some(high) = hex_digit(pair[0]) else {
                return Err(CredentialInputError::InvalidSecretEncoding);
            };
            let Some(low) = hex_digit(pair[1]) else {
                return Err(CredentialInputError::InvalidSecretEncoding);
            };
            bytes.push((high << 4) | low);
        }
        Self::new(bytes)
    }
}

const fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(..)")
    }
}

#[cfg(test)]
mod tests {
    use crate::{CredentialInputError, MAX_SECRET_BYTES, Secret};

    #[test]
    fn enforces_non_empty_bounded_secret_values() {
        assert!(matches!(
            Secret::new(Vec::new()),
            Err(CredentialInputError::EmptySecret)
        ));
        assert!(matches!(
            Secret::new(vec![7; MAX_SECRET_BYTES + 1]),
            Err(CredentialInputError::SecretTooLarge)
        ));
        assert_eq!(
            Secret::new(vec![0xA5, 0x5A])
                .expect("bounded secret is valid")
                .as_bytes(),
            &[0xA5, 0x5A]
        );
    }

    #[test]
    fn debug_output_redacts_secret_bytes() {
        let secret = Secret::new(b"do-not-render".to_vec()).expect("fixture is valid");
        assert_eq!(format!("{secret:?}"), "Secret(..)");
    }

    #[test]
    fn lowercase_hex_round_trips_arbitrary_secret_bytes() {
        let secret = Secret::new(vec![0, 0x10, 0xAF, 0xFF]).expect("fixture is valid");
        assert_eq!(secret.to_lower_hex(), "0010afff");
        assert_eq!(
            Secret::from_lower_hex("0010afff")
                .expect("canonical encoding is valid")
                .as_bytes(),
            &[0, 0x10, 0xAF, 0xFF]
        );
    }

    #[test]
    fn rejects_noncanonical_or_oversized_hex_secret_values() {
        for value in ["0", "0A", "zz", "00 ", "00\n"] {
            assert!(matches!(
                Secret::from_lower_hex(value),
                Err(CredentialInputError::InvalidSecretEncoding)
            ));
        }
        assert!(matches!(
            Secret::from_lower_hex(&"00".repeat(MAX_SECRET_BYTES + 1)),
            Err(CredentialInputError::InvalidSecretEncoding)
        ));
    }
}
