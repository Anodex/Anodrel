//! The portable service seam for one selected-output binary replacement.

use std::fmt;

use crate::{FileBinaryData, SaveReference};

/// Writes bounded decoded bytes through one selected-output reference.
///
/// Implementations receive no path, native handle, or encoded protocol value.
/// They consume the retained output object before mutating it and can discard a
/// reference when a later protocol validation step fails.
pub trait FileBinaryWriteService: fmt::Debug + Send {
    /// Consumes the retained output object and writes its bounded bytes.
    fn write_binary(
        &self,
        reference: &SaveReference,
        data: &FileBinaryData,
    ) -> Result<(), FileBinaryWriteServiceError>;

    /// Consumes and drops one retained output object without writing it.
    ///
    /// The core calls this after authorization when encoded data is malformed
    /// or over the bound, so a save reference cannot be retried with different
    /// data after a failed binary-write attempt.
    fn discard(&self, reference: &SaveReference);
}

/// A safe selected-output binary-write failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileBinaryWriteServiceError {
    /// The selected output reference was absent, expired, or could not be written.
    Unavailable,
}

impl fmt::Display for FileBinaryWriteServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected output binary data is unavailable")
    }
}

impl std::error::Error for FileBinaryWriteServiceError {}

/// A safe default service for hosts that do not expose binary output writes.
#[derive(Debug, Default)]
pub struct UnavailableFileBinaryWriteService;

impl FileBinaryWriteService for UnavailableFileBinaryWriteService {
    fn write_binary(
        &self,
        _reference: &SaveReference,
        _data: &FileBinaryData,
    ) -> Result<(), FileBinaryWriteServiceError> {
        Err(FileBinaryWriteServiceError::Unavailable)
    }

    fn discard(&self, _reference: &SaveReference) {}
}
