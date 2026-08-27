//! Opaque references to host-retained selected-folder state.

use std::fmt;

use crate::FOLDER_REFERENCE_BYTES;

/// An opaque Version 1 base64url reference to host-retained selected-folder state.
///
/// A host adapter derives this from 128 bits of cryptographically secure random
/// data only after it captures the selected folder's native identity. It is
/// distinct from file-read and output-write references by type.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FolderReference(String);

impl FolderReference {
    /// Validates one exact opaque folder reference.
    ///
    /// Validation deliberately does not generate a value or access a folder.
    pub fn new(value: impl Into<String>) -> Result<Self, FolderReferenceError> {
        let value = value.into();
        if value.len() != FOLDER_REFERENCE_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(FolderReferenceError::Invalid);
        }
        Ok(Self(value))
    }

    /// Returns the opaque reference for the protocol boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A safe failure while validating an opaque folder reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderReferenceError {
    /// The reference was not an exact Version 1 base64url value.
    Invalid,
}

impl fmt::Display for FolderReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("folder reference is invalid")
    }
}

impl std::error::Error for FolderReferenceError {}
