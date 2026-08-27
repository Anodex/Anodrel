//! Validated bounded folder-entry snapshot values.

use std::fmt;

/// Maximum direct entries returned from one consumed folder reference.
pub const MAX_FOLDER_ENTRIES: usize = 32;
/// Maximum UTF-8 bytes in one direct-entry name.
pub const MAX_FOLDER_ENTRY_NAME_BYTES: usize = 1024;

/// The only classifications exposed for one direct folder entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderEntryKind {
    /// A regular non-reparse file.
    File,
    /// A regular non-reparse directory.
    Directory,
    /// A reparse point or entry that cannot be safely classified.
    Other,
}

/// One direct child name and its conservative kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderEntry {
    name: String,
    kind: FolderEntryKind,
}

impl FolderEntry {
    /// Validates one direct child name and attaches its conservative kind.
    pub fn new(name: impl Into<String>, kind: FolderEntryKind) -> Result<Self, FolderEntryError> {
        let name = name.into();
        if !is_entry_name(&name) {
            return Err(FolderEntryError::InvalidName);
        }
        Ok(Self { name, kind })
    }

    /// Returns the entry's direct child name, never a path.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the entry's conservative classification.
    #[must_use]
    pub const fn kind(&self) -> FolderEntryKind {
        self.kind
    }
}

/// A bounded direct-entry snapshot from one selected folder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderEntries {
    entries: Vec<FolderEntry>,
    complete: bool,
}

impl FolderEntries {
    /// Builds a bounded direct-entry snapshot.
    ///
    /// `complete` is false when the adapter found more direct entries than the
    /// bounded response can return. Entry order is intentionally unspecified.
    pub fn new(entries: Vec<FolderEntry>, complete: bool) -> Result<Self, FolderEntriesError> {
        if entries.len() > MAX_FOLDER_ENTRIES {
            return Err(FolderEntriesError::TooManyEntries);
        }
        if has_duplicate_names(&entries) {
            return Err(FolderEntriesError::DuplicateName);
        }
        Ok(Self { entries, complete })
    }

    /// Returns the bounded direct entries in adapter-provided order.
    #[must_use]
    pub fn entries(&self) -> &[FolderEntry] {
        &self.entries
    }

    /// Returns whether every direct entry fit in this snapshot.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }
}

/// A safe direct-entry name validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderEntryError {
    /// The value was empty, reserved, too long, or contained a separator/control character.
    InvalidName,
}

impl fmt::Display for FolderEntryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("folder entry is invalid")
    }
}

impl std::error::Error for FolderEntryError {}

/// A safe bounded folder-entry snapshot failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderEntriesError {
    /// The adapter attempted to expose more than the fixed entry limit.
    TooManyEntries,
    /// The adapter attempted to expose the same exact child name twice.
    DuplicateName,
}

impl fmt::Display for FolderEntriesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("folder entries are invalid")
    }
}

impl std::error::Error for FolderEntriesError {}

fn is_entry_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FOLDER_ENTRY_NAME_BYTES
        && !matches!(value, "." | "..")
        && value
            .chars()
            .all(|character| !character.is_control() && !matches!(character, '/' | '\\'))
}

fn has_duplicate_names(entries: &[FolderEntry]) -> bool {
    entries.iter().enumerate().any(|(index, entry)| {
        entries[index + 1..]
            .iter()
            .any(|other| other.name == entry.name)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        FolderEntries, FolderEntriesError, FolderEntry, FolderEntryError, FolderEntryKind,
        MAX_FOLDER_ENTRIES, MAX_FOLDER_ENTRY_NAME_BYTES,
    };

    #[test]
    fn accepts_a_direct_entry_name_and_conservative_kind() {
        let entry = FolderEntry::new("notes.txt", FolderEntryKind::File).expect("entry is valid");
        assert_eq!(entry.name(), "notes.txt");
        assert_eq!(entry.kind(), FolderEntryKind::File);
    }

    #[test]
    fn rejects_paths_reserved_values_and_control_characters() {
        for name in ["", ".", "..", "child/name", "child\\name", "child\nname"] {
            assert_eq!(
                FolderEntry::new(name, FolderEntryKind::Other),
                Err(FolderEntryError::InvalidName),
                "{name:?} must not become filesystem authority"
            );
        }
        assert_eq!(
            FolderEntry::new(
                "a".repeat(MAX_FOLDER_ENTRY_NAME_BYTES + 1),
                FolderEntryKind::Other,
            ),
            Err(FolderEntryError::InvalidName)
        );
    }

    #[test]
    fn preserves_the_entry_bound_completion_signal_and_unique_names() {
        let entries = (0..MAX_FOLDER_ENTRIES)
            .map(|index| {
                FolderEntry::new(format!("{index}.txt"), FolderEntryKind::File)
                    .expect("entry is valid")
            })
            .collect();
        let snapshot = FolderEntries::new(entries, false).expect("snapshot is bounded");
        assert_eq!(snapshot.entries().len(), MAX_FOLDER_ENTRIES);
        assert!(!snapshot.is_complete());

        let duplicate = FolderEntry::new("same", FolderEntryKind::File).expect("entry is valid");
        assert_eq!(
            FolderEntries::new(vec![duplicate.clone(), duplicate], true),
            Err(FolderEntriesError::DuplicateName)
        );
    }
}
