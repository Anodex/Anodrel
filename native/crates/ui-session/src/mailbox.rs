//! Bounded, coalescing cross-thread delivery for UI document snapshots.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::UiDocumentSnapshot;

/// A per-session slot that retains only the newest pending document snapshot.
///
/// Publishing never waits for a receiver and stores no history. A host owns the
/// notification or polling mechanism used to consume this portable value.
#[derive(Clone, Debug, Default)]
pub struct UiDocumentMailbox {
    pending: Arc<Mutex<Option<UiDocumentSnapshot>>>,
}

impl UiDocumentMailbox {
    /// Creates an empty mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Retains `snapshot` only when it is newer than the pending value.
    pub fn publish(&self, snapshot: UiDocumentSnapshot) {
        let pending = &mut *lock(&self.pending);
        let should_replace = pending
            .as_ref()
            .is_none_or(|current| snapshot.revision() > current.revision());
        if should_replace {
            *pending = Some(snapshot);
        }
    }

    /// Takes and clears the one newest pending snapshot, if any.
    pub fn take(&self) -> Option<UiDocumentSnapshot> {
        lock(&self.pending).take()
    }
}

fn lock(value: &Mutex<Option<UiDocumentSnapshot>>) -> MutexGuard<'_, Option<UiDocumentSnapshot>> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
