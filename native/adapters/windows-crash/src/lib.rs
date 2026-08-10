#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows store for host crash records.
//!
//! Writes one bounded record per contained panic into the host's own
//! diagnostics location, keeps a fixed number of the most recent, and reports
//! nothing to anybody. See `docs/CRASH_REPORTS.md` and Decision 0065.
//!
//! Every failure resolves to a category carrying no path and no native status.
//! The caller is a host that is already shutting down after a defect, so this
//! adapter must never panic, block, or make the failure worse than it is.

mod raw;

use std::{fmt, path::PathBuf};

use anodrel_crash::{CrashRecord, CrashReportError, CrashReporter, serialize};
use anodrel_paths::HostDirectories;

/// Records kept before the oldest is removed.
///
/// Small on purpose. A host that panics repeatedly must not be able to fill a
/// disk, and the interesting record is almost always the most recent one.
pub const RETAINED_RECORDS: usize = 8;

const FILE_PREFIX: &str = "crash-";
const FILE_SUFFIX: &str = ".anodrel.v1";

/// Host-owned Windows crash-record store.
pub struct WindowsCrashStore {
    directory: PathBuf,
}

impl WindowsCrashStore {
    /// Builds a store over the host's own diagnostics location.
    #[must_use]
    pub fn new(directories: &HostDirectories) -> Self {
        Self {
            directory: directories.logs().to_path_buf(),
        }
    }

    /// Returns the sequences of the records already stored, ascending.
    ///
    /// Names that do not match the record pattern are ignored rather than
    /// treated as an error. The location is the host's, but it is an ordinary
    /// directory on a user's machine and may contain anything.
    fn stored_sequences(&self) -> Result<Vec<u64>, CrashReportError> {
        let names =
            raw::regular_file_names(&self.directory).map_err(|()| CrashReportError::WriteFailed)?;
        let mut sequences: Vec<u64> = names.iter().filter_map(|name| sequence_of(name)).collect();
        sequences.sort_unstable();
        Ok(sequences)
    }

    /// Removes the oldest records until at most `RETAINED_RECORDS` remain.
    ///
    /// Runs after the new record is written, not before: losing an old record
    /// to make room for one that then fails to write would be the worst of both.
    fn evict_oldest(&self, sequences: &[u64]) {
        let excess = sequences.len().saturating_sub(RETAINED_RECORDS);
        for sequence in &sequences[..excess] {
            // Best effort. A record that cannot be removed is left alone; the
            // next report tries again, and the bound is a policy rather than an
            // invariant anything depends on.
            let _ = raw::delete_file(&self.directory.join(file_name(*sequence)));
        }
    }
}

impl CrashReporter for WindowsCrashStore {
    fn report(&self, record: &CrashRecord) -> Result<u64, CrashReportError> {
        raw::ensure_directory_tree(&self.directory)
            .map_err(|()| CrashReportError::LocationUnavailable)?;

        let mut sequences = self.stored_sequences()?;
        let sequence = sequences.last().copied().unwrap_or(0).saturating_add(1);
        let text = serialize(record, sequence)?;

        raw::write_new_file(&self.directory.join(file_name(sequence)), text.as_bytes())
            .map_err(|()| CrashReportError::WriteFailed)?;

        sequences.push(sequence);
        self.evict_oldest(&sequences);
        Ok(sequence)
    }
}

impl fmt::Debug for WindowsCrashStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never the path. A Debug line is the easiest way for an absolute path
        // to end up somewhere it was never meant to be.
        formatter.write_str("WindowsCrashStore(..)")
    }
}

/// Returns the file name holding the record with this sequence.
fn file_name(sequence: u64) -> String {
    format!("{FILE_PREFIX}{sequence}{FILE_SUFFIX}")
}

/// Returns the sequence a record file name carries, if it is one.
fn sequence_of(name: &str) -> Option<u64> {
    name.strip_prefix(FILE_PREFIX)?
        .strip_suffix(FILE_SUFFIX)?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use anodrel_crash::{CrashRecord, CrashReporter, CrashSite, CrashSurface};
    use anodrel_paths::HostDirectories;

    use super::{RETAINED_RECORDS, WindowsCrashStore, file_name, raw, sequence_of};

    /// A private local-data root for one test, removed on drop.
    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!("anodrel-crash-test-{label}"));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("the test root is creatable");
            Self(path)
        }

        fn store(&self) -> (WindowsCrashStore, PathBuf) {
            let directories =
                HostDirectories::from_local_data_root(&self.0).expect("an absolute root");
            (
                WindowsCrashStore::new(&directories),
                directories.logs().to_path_buf(),
            )
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn record() -> CrashRecord {
        CrashRecord::new(
            CrashSite::WindowProcedure,
            CrashSurface::StartupLab,
            "0.1.0",
        )
    }

    #[test]
    fn the_first_report_creates_the_location_and_writes_one_record() {
        let root = TempRoot::new("first");
        let (store, logs) = root.store();
        assert!(!logs.exists(), "the location must not exist beforehand");

        let sequence = store.report(&record()).expect("the first record is stored");
        assert_eq!(sequence, 1);

        let written =
            std::fs::read_to_string(logs.join(file_name(1))).expect("the record is there");
        assert_eq!(
            written,
            "format=anodrel.crash.v1\n\
             site=window-procedure\n\
             surface=startup-lab\n\
             hostVersion=0.1.0\n\
             sequence=1\n"
        );
    }

    #[test]
    fn a_later_process_continues_the_sequence_rather_than_colliding() {
        // Every report here is a separate store, standing in for a separate run
        // of the host. This is the case that made the sequence belong to the
        // store: a process-local counter would write crash-1 every time.
        let root = TempRoot::new("continues");
        for expected in 1..=3 {
            let (store, _) = root.store();
            assert_eq!(
                store.report(&record()).expect("each record is stored"),
                expected
            );
        }
        let (_, logs) = root.store();
        for sequence in 1..=3 {
            assert!(logs.join(file_name(sequence)).exists());
        }
    }

    #[test]
    fn retention_keeps_the_newest_and_removes_the_oldest() {
        let root = TempRoot::new("retention");
        let (store, logs) = root.store();
        let total = RETAINED_RECORDS as u64 + 3;
        for expected in 1..=total {
            assert_eq!(store.report(&record()).expect("stored"), expected);
        }

        let mut remaining: Vec<u64> = raw::regular_file_names(&logs)
            .expect("the location lists")
            .iter()
            .filter_map(|name| sequence_of(name))
            .collect();
        remaining.sort_unstable();
        let newest: Vec<u64> = ((total - RETAINED_RECORDS as u64 + 1)..=total).collect();
        assert_eq!(remaining, newest);
    }

    #[test]
    fn an_unrelated_file_in_the_location_is_ignored() {
        // The host's directory is still an ordinary directory on someone's
        // machine. A stray file must not become a sequence or be deleted.
        let root = TempRoot::new("unrelated");
        let (store, logs) = root.store();
        std::fs::create_dir_all(&logs).expect("the location is creatable");
        let stray = logs.join("notes.txt");
        std::fs::write(&stray, "not a record").expect("the stray file is writable");
        std::fs::write(logs.join("crash-not-a-number.anodrel.v1"), "also not")
            .expect("the second stray file is writable");

        assert_eq!(store.report(&record()).expect("stored"), 1);
        assert!(stray.exists(), "an unrelated file was removed");
    }

    #[test]
    fn a_location_blocked_by_a_file_fails_without_panicking() {
        let root = TempRoot::new("blocked");
        let (store, logs) = root.store();
        // Occupy the directory's own name with a file, so creating the tree
        // cannot succeed.
        std::fs::create_dir_all(logs.parent().expect("logs has a parent"))
            .expect("the parent is creatable");
        std::fs::write(&logs, "in the way").expect("the blocking file is writable");

        assert_eq!(
            store.report(&record()),
            Err(anodrel_crash::CrashReportError::LocationUnavailable)
        );
    }

    #[test]
    fn debug_output_never_reveals_the_location() {
        let root = TempRoot::new("debug");
        let (store, _) = root.store();
        assert_eq!(format!("{store:?}"), "WindowsCrashStore(..)");
    }

    #[test]
    fn a_file_name_round_trips_its_sequence() {
        for sequence in [1_u64, 9, 10, u64::MAX] {
            assert_eq!(sequence_of(&file_name(sequence)), Some(sequence));
        }
        for name in [
            "crash-.anodrel.v1",
            "crash--1.anodrel.v1",
            "crash-1.anodrel.v2",
            "crash-1",
            "1.anodrel.v1",
            "notes.txt",
        ] {
            assert_eq!(sequence_of(name), None, "{name:?} parsed as a record");
        }
    }

    #[test]
    fn listing_a_missing_location_is_empty_rather_than_a_failure() {
        let missing = Path::new(r"C:\Anodrel-crash-test-location-that-does-not-exist\logs");
        assert_eq!(raw::regular_file_names(missing), Ok(Vec::new()));
    }
}
