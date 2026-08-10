#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Bounded host-only records of a contained panic.
//!
//! This crate defines what a crash record may say and how it is written down.
//! It performs no I/O and reaches no operating-system API; a host supplies a
//! [`CrashReporter`] that does. See `docs/CRASH_REPORTS.md` for the boundary
//! and Decision 0065 for why it is drawn where it is.
//!
//! Nothing here accepts caller-supplied text. Every field of a record comes
//! from a closed catalogue or a constant, which is what keeps a panic payload —
//! whose contents are unknown at compile time — out of a file on disk.

mod record;
mod serialize;

pub use record::{CrashRecord, CrashSite, CrashSurface, FORMAT};
pub use serialize::{MAX_RECORD_BYTES, serialize};

/// Why a crash record could not be written down.
///
/// These categories exist to be counted and tested, not displayed. Reporting is
/// silent: see [`CrashReporter`]. None of them carries a path, a native status
/// code, or anything derived from the failure being reported.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrashReportError {
    /// The host could not resolve or create its diagnostics location.
    LocationUnavailable,
    /// The record could not be written completely.
    WriteFailed,
    /// The serialized record exceeded [`MAX_RECORD_BYTES`].
    ///
    /// A record is refused rather than truncated. A truncated record is one
    /// whose meaning cannot be trusted, which is worse than no record.
    RecordTooLarge,
    /// A field held something the record format cannot carry.
    ///
    /// Only the host version can reach this: every other field is a catalogue
    /// value or a counter. Nothing supplies a version that would trip it today,
    /// and the category exists so that stays true if one ever stops being a
    /// compile-time constant.
    RecordMalformed,
}

/// A store that keeps crash records.
///
/// A host implements this over its own filesystem. The trait takes a finished
/// [`CrashRecord`] and never the panic that produced it, so no implementation
/// can be handed a payload to be tempted by.
///
/// The store assigns each record's sequence and owns its own retention. Order
/// is a property of the store, not of the crash — see [`CrashRecord`].
///
/// An implementation must not panic, block indefinitely, or report a failure to
/// a user, an application, or the diagnostic ledger. It is called while the
/// host is already shutting down after a contained defect, and a reporter that
/// makes noise there turns a handled defect into an unhandled one.
pub trait CrashReporter {
    /// Records one crash, returning the sequence the store assigned it.
    ///
    /// # Errors
    ///
    /// Returns a [`CrashReportError`] category when the record could not be
    /// written. The caller is expected to ignore it outside tests.
    fn report(&self, record: &CrashRecord) -> Result<u64, CrashReportError>;
}
