#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only Windows Credential Manager access.
//!
//! The adapter reads, writes, and deletes only exact generic targets derived
//! from a validated application identity. It has no public protocol surface,
//! enumeration, prompt, or diagnostic output. See `docs/CREDENTIALS.md`.

mod raw;

use std::fmt;

use anodrel_application::ApplicationIdentity;
use anodrel_credentials::{
    CredentialInputError, CredentialName, CredentialService, CredentialServiceError,
    CredentialTarget, Secret,
};

/// One Windows Credential Manager service bound to a host-validated
/// application identity. Protocol callers never provide this identity.
#[derive(Clone, Debug)]
pub struct WindowsCredentialService {
    identity: ApplicationIdentity,
}

impl WindowsCredentialService {
    /// Binds a Windows credential service to one validated application.
    pub fn new(identity: ApplicationIdentity) -> Self {
        Self { identity }
    }
}

impl CredentialService for WindowsCredentialService {
    fn read(&self, name: &CredentialName) -> Result<Secret, CredentialServiceError> {
        read(&self.identity, name).map_err(service_error)
    }

    fn write(&self, name: &CredentialName, secret: &Secret) -> Result<(), CredentialServiceError> {
        write(&self.identity, name, secret).map_err(service_error)
    }

    fn delete(&self, name: &CredentialName) -> Result<bool, CredentialServiceError> {
        delete(&self.identity, name).map_err(service_error)
    }
}

/// Writes one bounded secret to its exact derived Windows credential target.
pub fn write(
    identity: &ApplicationIdentity,
    name: &CredentialName,
    secret: &Secret,
) -> Result<(), CredentialStoreError> {
    let target = target(identity, name)?;
    raw::write(target.as_str(), secret.as_bytes()).map_err(CredentialStoreError::from)
}

/// Reads one bounded secret from its exact derived Windows credential target.
pub fn read(
    identity: &ApplicationIdentity,
    name: &CredentialName,
) -> Result<Secret, CredentialStoreError> {
    let target = target(identity, name)?;
    let bytes = raw::read(target.as_str()).map_err(CredentialStoreError::from)?;
    Secret::new(bytes).map_err(|_| CredentialStoreError::StoredSecretInvalid)
}

/// Deletes one exact derived Windows credential target.
///
/// Returns `false` when no credential existed, making controlled cleanup
/// idempotent without exposing store enumeration.
pub fn delete(
    identity: &ApplicationIdentity,
    name: &CredentialName,
) -> Result<bool, CredentialStoreError> {
    let target = target(identity, name)?;
    raw::delete(target.as_str()).map_err(CredentialStoreError::from)
}

fn target(
    identity: &ApplicationIdentity,
    name: &CredentialName,
) -> Result<CredentialTarget, CredentialStoreError> {
    CredentialTarget::for_application(identity, name).map_err(CredentialStoreError::Input)
}

/// A safe category for a Windows credential-store failure.
#[derive(Debug)]
pub enum CredentialStoreError {
    Input(CredentialInputError),
    NotFound,
    AccessDenied,
    Unavailable,
    StoredSecretInvalid,
}

impl From<raw::CredentialManagerError> for CredentialStoreError {
    fn from(error: raw::CredentialManagerError) -> Self {
        match error {
            raw::CredentialManagerError::NotFound => Self::NotFound,
            raw::CredentialManagerError::AccessDenied => Self::AccessDenied,
            raw::CredentialManagerError::Unavailable => Self::Unavailable,
            raw::CredentialManagerError::StoredSecretInvalid => Self::StoredSecretInvalid,
        }
    }
}

impl fmt::Display for CredentialStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Input(_) => "credential input is invalid",
            Self::NotFound => "credential was not found",
            Self::AccessDenied => "credential store access was denied",
            Self::Unavailable => "Windows credential store is unavailable",
            Self::StoredSecretInvalid => "stored credential secret is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CredentialStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Input(error) => Some(error),
            Self::NotFound | Self::AccessDenied | Self::Unavailable | Self::StoredSecretInvalid => {
                None
            }
        }
    }
}

fn service_error(error: CredentialStoreError) -> CredentialServiceError {
    match error {
        CredentialStoreError::Input(_) | CredentialStoreError::StoredSecretInvalid => {
            CredentialServiceError::StoredSecretInvalid
        }
        CredentialStoreError::NotFound => CredentialServiceError::NotFound,
        CredentialStoreError::AccessDenied => CredentialServiceError::AccessDenied,
        CredentialStoreError::Unavailable => CredentialServiceError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anodrel_application::{ApplicationIdentity, ApplicationManifest};
    use anodrel_credentials::{CredentialName, CredentialService, Secret};

    use super::{CredentialStoreError, WindowsCredentialService, delete, read};

    struct CredentialCleanup {
        identity: ApplicationIdentity,
        name: CredentialName,
    }

    impl Drop for CredentialCleanup {
        fn drop(&mut self) {
            let _ = delete(&self.identity, &self.name);
        }
    }

    fn identity() -> ApplicationIdentity {
        ApplicationManifest::parse(
            r#"{
                "manifestVersion": { "major": 1, "minor": 0 },
                "applicationId": "org.anodrel.windows-credentials-test",
                "displayName": "Windows Credentials Test",
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
    fn writes_reads_and_removes_one_scoped_test_credential() {
        let identity = identity();
        let name = CredentialName::parse(&format!(
            "roundtrip-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is after epoch")
                .as_nanos()
        ))
        .expect("test name is valid");
        let cleanup = CredentialCleanup {
            identity: identity.clone(),
            name: name.clone(),
        };
        assert!(!delete(&identity, &name).expect("unique target has no stale credential"));

        let expected = b"anodrel credential roundtrip";
        let secret = Secret::new(expected.to_vec()).expect("fixture secret is valid");
        let service = WindowsCredentialService::new(identity.clone());
        service
            .write(&name, &secret)
            .expect("Windows writes the scoped credential");
        let loaded = service
            .read(&name)
            .expect("Windows reads the scoped credential");
        assert_eq!(loaded.as_bytes(), expected);

        assert!(
            service
                .delete(&name)
                .expect("Windows removes the scoped credential")
        );
        assert!(matches!(
            read(&identity, &name),
            Err(CredentialStoreError::NotFound)
        ));
        drop(cleanup);
    }
}
