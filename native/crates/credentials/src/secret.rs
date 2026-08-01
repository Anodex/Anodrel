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
}
