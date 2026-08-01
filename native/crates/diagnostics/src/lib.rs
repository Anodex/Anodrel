#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Closed, internal diagnostic events.
//!
//! The diagnostic ledger is intentionally not a general logging API. It holds
//! only events from the closed Event catalogue, never application input,
//! operating-system errors, paths, secrets, or persisted state. See
//! docs/LOGGING.md for the public boundary description.

use std::collections::VecDeque;

/// Maximum number of process-local diagnostic records retained at once.
pub const MAX_ENTRIES: usize = 64;

/// Severity assigned to every event in the current closed catalogue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Level {
    /// An expected host lifecycle event.
    Info,
}

impl Level {
    /// Returns the stable text label for this severity.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
        }
    }
}

/// A fixed host lifecycle event.
///
/// This enum deliberately carries no caller-supplied payload. Adding a variant
/// changes the displayed diagnostic surface and must follow the compatibility
/// gate in docs/LOGGING.md.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    /// The application package passed host verification.
    PackageVerified,
    /// The internal core health request completed.
    CoreHealthChecked,
    /// The private pipe loopback check completed.
    PipeLoopbackChecked,
    /// The Startup Lab was allowed to open after preflight.
    StartupLabAuthorized,
}

impl Event {
    /// Returns the fixed component label for this event.
    #[must_use]
    pub const fn component(self) -> &'static str {
        match self {
            Self::PackageVerified => "package",
            Self::CoreHealthChecked => "core",
            Self::PipeLoopbackChecked => "transport",
            Self::StartupLabAuthorized => "host",
        }
    }

    /// Returns the fixed human-readable message for this event.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::PackageVerified => "Package verification completed.",
            Self::CoreHealthChecked => "Internal platform.health check completed.",
            Self::PipeLoopbackChecked => "Private pipe loopback check completed.",
            Self::StartupLabAuthorized => "Startup Lab opened after preflight.",
        }
    }
}

/// One immutable record from the host diagnostic ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entry {
    sequence: u64,
    event: Event,
}

impl Entry {
    /// Returns the process-local order assigned by the ledger.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// Returns the fixed event category.
    #[must_use]
    pub const fn event(self) -> Event {
        self.event
    }

    /// Returns the severity for this record.
    #[must_use]
    pub const fn level(self) -> Level {
        Level::Info
    }

    /// Returns the fixed component label for this record.
    #[must_use]
    pub const fn component(self) -> &'static str {
        self.event.component()
    }

    /// Returns the fixed message for this record.
    #[must_use]
    pub const fn message(self) -> &'static str {
        self.event.message()
    }
}

/// Error returned only if a process records more than u64 maximum sequence values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordError {
    /// The process-local sequence number cannot advance further.
    SequenceExhausted,
}

/// Bounded process-local history of fixed host diagnostic events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogBook {
    next_sequence: u64,
    entries: VecDeque<Entry>,
}

impl LogBook {
    /// Creates an empty ledger with the documented retention bound.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_sequence: 0,
            entries: VecDeque::with_capacity(MAX_ENTRIES),
        }
    }

    /// Records one fixed host event, discarding the oldest retained entry when full.
    pub fn record(&mut self, event: Event) -> Result<(), RecordError> {
        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(RecordError::SequenceExhausted)?;
        self.next_sequence = next_sequence;
        if self.entries.len() == MAX_ENTRIES {
            let _ = self.entries.pop_front();
        }
        self.entries.push_back(Entry {
            sequence: next_sequence,
            event,
        });
        Ok(())
    }

    /// Returns the retained records in ascending process-local order.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = &Entry> {
        self.entries.iter()
    }

    /// Returns the number of records currently retained.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Reports whether this ledger currently has no retained records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for LogBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, Level, LogBook, MAX_ENTRIES};

    #[test]
    fn fixed_events_produce_the_documented_record_fields() {
        let events = [
            Event::PackageVerified,
            Event::CoreHealthChecked,
            Event::PipeLoopbackChecked,
            Event::StartupLabAuthorized,
        ];
        let mut log = LogBook::new();
        for event in events {
            log.record(event).expect("test sequence capacity");
        }

        let entries: Vec<_> = log.entries().copied().collect();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].sequence(), 1);
        assert_eq!(entries[3].sequence(), 4);
        assert_eq!(entries[1].level(), Level::Info);
        assert_eq!(entries[2].component(), "transport");
        assert_eq!(entries[3].message(), "Startup Lab opened after preflight.");
    }

    #[test]
    fn fixed_catalogue_cannot_introduce_path_or_control_characters() {
        for event in [
            Event::PackageVerified,
            Event::CoreHealthChecked,
            Event::PipeLoopbackChecked,
            Event::StartupLabAuthorized,
        ] {
            for field in [event.component(), event.message()] {
                assert!(!field.contains(char::from(92)));
                assert!(!field.contains('/'));
                assert!(!field.contains(':'));
                assert!(!field.chars().any(char::is_control));
            }
        }
    }

    #[test]
    fn the_oldest_event_drops_when_the_bound_is_exceeded() {
        let mut log = LogBook::new();
        for _ in 0..=MAX_ENTRIES {
            log.record(Event::PackageVerified)
                .expect("test sequence capacity");
        }

        assert_eq!(log.len(), MAX_ENTRIES);
        assert_eq!(
            log.entries()
                .next()
                .expect("retained first entry")
                .sequence(),
            2
        );
        assert_eq!(
            log.entries()
                .last()
                .expect("retained final entry")
                .sequence(),
            (MAX_ENTRIES + 1) as u64
        );
    }
}
