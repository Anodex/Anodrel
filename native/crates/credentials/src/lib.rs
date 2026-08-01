#![forbid(unsafe_code)]

//! Host-only credential names, targets, and opaque secret values.
//!
//! This crate has no operating-system calls and no public application protocol.
//! A native credential-store adapter owns persistence. See `docs/CREDENTIALS.md`.

mod name;
mod secret;
mod target;

use std::fmt;

pub use name::CredentialName;
pub use secret::Secret;
pub use target::CredentialTarget;

/// Largest v1 credential name in bytes.
pub const MAX_CREDENTIAL_NAME_BYTES: usize = 64;

/// Largest v1 secret, deliberately below Windows' generic-credential limit.
pub const MAX_SECRET_BYTES: usize = 2 * 1024;

/// A safe validation category for host-only credential inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialInputError {
    InvalidCredentialName,
    EmptySecret,
    SecretTooLarge,
    InvalidSecretEncoding,
    InvalidApplicationIdentity,
}

impl fmt::Display for CredentialInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCredentialName => "credential name is invalid",
            Self::EmptySecret => "credential secret is empty",
            Self::SecretTooLarge => "credential secret exceeds its limit",
            Self::InvalidSecretEncoding => "credential secret encoding is invalid",
            Self::InvalidApplicationIdentity => "application identity is invalid for credentials",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CredentialInputError {}

/// Host-injected exact-target credential operations for one application
/// identity. The implementation, not a protocol caller, owns that identity.
pub trait CredentialService: std::fmt::Debug + Send {
    /// Reads one existing credential by its validated local name.
    fn read(&self, name: &CredentialName) -> Result<Secret, CredentialServiceError>;

    /// Replaces one credential by its validated local name.
    fn write(&self, name: &CredentialName, secret: &Secret) -> Result<(), CredentialServiceError>;

    /// Deletes one credential by its validated local name.
    ///
    /// `false` means the exact credential was not present.
    fn delete(&self, name: &CredentialName) -> Result<bool, CredentialServiceError>;
}

/// Safe categories a native credential service can return to the host core.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialServiceError {
    /// The exact credential was not present.
    NotFound,
    /// The current process cannot access the exact credential.
    AccessDenied,
    /// The selected operating-system credential service is unavailable.
    Unavailable,
    /// A persisted credential violates Anodrel's bounded secret contract.
    StoredSecretInvalid,
}
