//! Bounded semantic input handoff from one native UI view to its session core.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};

use anodrel_ui::UiEvent;

use crate::UiDocumentRevision;

/// The fixed maximum number of pending semantic input candidates per session.
pub const UI_INPUT_QUEUE_CAPACITY: usize = 32;

/// One native-layout-derived semantic input candidate awaiting session validation.
#[derive(Clone, Debug)]
pub struct UiInputCandidate {
    revision: UiDocumentRevision,
    event: UiEvent,
}

impl UiInputCandidate {
    /// Builds one candidate from the revision used by a host layout and event.
    #[must_use]
    pub const fn new(revision: UiDocumentRevision, event: UiEvent) -> Self {
        Self { revision, event }
    }

    /// Splits this candidate into its revision and semantic event.
    #[must_use]
    pub fn into_parts(self) -> (UiDocumentRevision, UiEvent) {
        (self.revision, self.event)
    }
}

/// A bounded per-session queue of native semantic input candidates.
///
/// New candidates are dropped when the fixed queue is full. A consumer receives
/// the exact drop count while draining the queue, so input loss is never silent.
#[derive(Clone, Debug, Default)]
pub struct UiInputMailbox {
    state: Arc<Mutex<UiInputMailboxState>>,
}

#[derive(Debug, Default)]
struct UiInputMailboxState {
    pending: VecDeque<UiInputCandidate>,
    dropped: u32,
}

impl UiInputMailbox {
    /// Creates an empty per-session input mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one candidate or records an overflow when the queue is full.
    pub fn push(&self, candidate: UiInputCandidate) {
        let state = &mut *lock(&self.state);
        if state.pending.len() == UI_INPUT_QUEUE_CAPACITY {
            state.dropped = state.dropped.saturating_add(1);
        } else {
            state.pending.push_back(candidate);
        }
    }

    /// Takes all pending candidates with the exact overflow count since the
    /// previous drain.
    #[must_use]
    pub fn drain(&self) -> UiInputBatch {
        let state = &mut *lock(&self.state);
        let pending = std::mem::take(&mut state.pending);
        let dropped = std::mem::take(&mut state.dropped);
        UiInputBatch {
            candidates: pending.into_iter().collect(),
            dropped,
        }
    }
}

/// One bounded batch taken from a [`UiInputMailbox`].
#[derive(Debug)]
pub struct UiInputBatch {
    candidates: Vec<UiInputCandidate>,
    dropped: u32,
}

impl UiInputBatch {
    /// Returns candidates in their original native input order.
    #[must_use]
    pub fn into_candidates(self) -> Vec<UiInputCandidate> {
        self.candidates
    }

    /// Returns the number of newer candidates dropped because the queue was full.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }
}

fn lock(value: &Mutex<UiInputMailboxState>) -> MutexGuard<'_, UiInputMailboxState> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
