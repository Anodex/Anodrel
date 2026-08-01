#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Bounded opaque application-state snapshots.
//!
//! This crate owns only the portable value and service boundary for one
//! application state snapshot. Implementations own identity isolation, storage
//! location, atomic replacement, recovery, and operating-system calls. It
//! deliberately provides no path, file, stream, or protocol API. See
//! `docs/STORAGE.md` and Decision 0051.

use std::fmt;

/// Maximum UTF-8 bytes in one complete application-state snapshot.
pub const MAX_STORAGE_SNAPSHOT_BYTES: usize = 256 * 1024;

/// One validated opaque application-state snapshot.
#[derive(Clone, Eq, PartialEq)]
pub struct StorageSnapshot(String);

impl StorageSnapshot {
    /// Builds a snapshot after enforcing the fixed UTF-8 byte limit.
    pub fn new(value: impl Into<String>) -> Result<Self, StorageInputError> {
        let value = value.into();
        if value.len() > MAX_STORAGE_SNAPSHOT_BYTES {
            return Err(StorageInputError::TooLarge);
        }
        Ok(Self(value))
    }

    /// Returns the complete opaque UTF-8 value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for StorageSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("StorageSnapshot(..)")
    }
}

/// The stable result of reading one application-state snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageRead {
    /// No committed snapshot exists for the application.
    Absent,
    /// A complete committed snapshot is available, including an empty value.
    Snapshot(StorageSnapshot),
}

/// Portable boundary implemented by one host-owned application-state adapter.
///
/// An implementation must derive its storage namespace from the host-validated
/// application identity. It must not accept a caller-selected path, expose a
/// file handle, return partial data, or retain unbounded history.
pub trait StorageService: fmt::Debug + Send {
    /// Reads the complete current snapshot, if one exists.
    fn read(&self) -> Result<StorageRead, StorageServiceError>;

    /// Atomically replaces the complete current snapshot.
    fn replace(&self, snapshot: &StorageSnapshot) -> Result<(), StorageServiceError>;

    /// Removes the current snapshot if one exists.
    fn clear(&self) -> Result<(), StorageServiceError>;
}

/// A safe failure category from a host storage adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageServiceError {
    /// The host cannot safely access the application-state location.
    Unavailable,
    /// Stored data is not a valid complete UTF-8 snapshot.
    StoredSnapshotInvalid,
    /// Stored data exceeds the fixed portable snapshot limit.
    StoredSnapshotTooLarge,
}

impl fmt::Display for StorageServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unavailable => "application state is unavailable",
            Self::StoredSnapshotInvalid => "stored application state is invalid",
            Self::StoredSnapshotTooLarge => "stored application state is too large",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for StorageServiceError {}

/// A stable validation failure before a host storage call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageInputError {
    /// The UTF-8 input exceeds the fixed snapshot limit.
    TooLarge,
}

impl fmt::Display for StorageInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("application-state snapshot exceeds the fixed size limit")
    }
}

impl std::error::Error for StorageInputError {}

#[cfg(test)]
mod tests {
    use super::{
        MAX_STORAGE_SNAPSHOT_BYTES, StorageInputError, StorageRead, StorageServiceError,
        StorageSnapshot,
    };

    #[test]
    fn preserves_the_difference_between_absent_and_empty_state() {
        let empty = StorageSnapshot::new("").expect("empty state is valid");
        assert_eq!(StorageRead::Absent, StorageRead::Absent);
        assert_eq!(
            StorageRead::Snapshot(empty),
            StorageRead::Snapshot(StorageSnapshot::new("").unwrap())
        );
    }

    #[test]
    fn accepts_utf8_at_the_limit_and_rejects_only_larger_values() {
        assert!(StorageSnapshot::new("x".repeat(MAX_STORAGE_SNAPSHOT_BYTES)).is_ok());
        assert_eq!(
            StorageSnapshot::new("x".repeat(MAX_STORAGE_SNAPSHOT_BYTES + 1)),
            Err(StorageInputError::TooLarge)
        );
    }

    #[test]
    fn debug_and_errors_do_not_expose_snapshot_content() {
        let snapshot = StorageSnapshot::new("private state").expect("fixture state is valid");
        assert_eq!(format!("{snapshot:?}"), "StorageSnapshot(..)");
        assert_eq!(
            StorageServiceError::Unavailable.to_string(),
            "application state is unavailable"
        );
    }
}
