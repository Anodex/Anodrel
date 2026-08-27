//! Read and write service seams for retained file selections.

use std::fmt;

use crate::{SaveReference, SelectionReference};

/// Reads bounded UTF-8 text from one session-bound selected-file reference.
///
/// Implementations must never accept a path, native handle, or caller-selected
/// filesystem scope. A missing reference is intentionally indistinguishable
/// from an unavailable host selection at this portable boundary.
pub trait FileTextService: fmt::Debug + Send {
    /// Consumes the reference's retained file object and returns bounded text.
    fn read_text(&self, reference: &SelectionReference) -> Result<String, FileTextServiceError>;
}

/// A safe selected-file text service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileTextServiceError {
    /// The selected reference was absent, expired, or could not be read.
    Unavailable,
    /// The retained file exceeded the fixed reader limit.
    TooLarge,
    /// The retained file did not contain valid UTF-8 text.
    InvalidText,
}

impl fmt::Display for FileTextServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected file text is unavailable")
    }
}

impl std::error::Error for FileTextServiceError {}

/// A safe default service for hosts that do not expose selected-file reads.
#[derive(Debug, Default)]
pub struct UnavailableFileTextService;

impl FileTextService for UnavailableFileTextService {
    fn read_text(&self, _reference: &SelectionReference) -> Result<String, FileTextServiceError> {
        Err(FileTextServiceError::Unavailable)
    }
}

/// Writes bounded UTF-8 text through one session-bound selected-output reference.
///
/// Implementations must never accept a path, native handle, or caller-selected
/// filesystem scope. A missing reference is intentionally indistinguishable
/// from an unavailable host output selection at this portable boundary.
pub trait FileTextWriteService: fmt::Debug + Send {
    /// Consumes the reference's retained output object and writes bounded text.
    fn write_text(
        &self,
        reference: &SaveReference,
        text: &str,
    ) -> Result<(), FileTextWriteServiceError>;
}

/// A safe selected-output text-write failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileTextWriteServiceError {
    /// The selected output reference was absent, expired, or could not be written.
    Unavailable,
    /// The supplied text exceeded the fixed writer limit.
    TooLarge,
}

impl fmt::Display for FileTextWriteServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selected output text is unavailable")
    }
}

impl std::error::Error for FileTextWriteServiceError {}

/// A safe default service for hosts that do not expose selected-file writes.
#[derive(Debug, Default)]
pub struct UnavailableFileTextWriteService;

impl FileTextWriteService for UnavailableFileTextWriteService {
    fn write_text(
        &self,
        _reference: &SaveReference,
        _text: &str,
    ) -> Result<(), FileTextWriteServiceError> {
        Err(FileTextWriteServiceError::Unavailable)
    }
}
