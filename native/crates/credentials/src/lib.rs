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
    InvalidApplicationIdentity,
}

impl fmt::Display for CredentialInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCredentialName => "credential name is invalid",
            Self::EmptySecret => "credential secret is empty",
            Self::SecretTooLarge => "credential secret exceeds its limit",
            Self::InvalidApplicationIdentity => "application identity is invalid for credentials",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CredentialInputError {}
