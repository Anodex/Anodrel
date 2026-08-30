//! Bounded semantic input handoff from one native UI view to its session core.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::SessionInteractionCandidate;

/// The fixed maximum number of pending semantic input candidates per session.
pub const UI_INPUT_QUEUE_CAPACITY: usize = 32;

/// A bounded per-session queue of native semantic interaction candidates.
///
/// New candidates are dropped when the fixed queue is full. A consumer receives
/// the exact drop count while draining the queue, so input loss is never silent.
#[derive(Clone, Debug, Default)]
pub struct UiInputMailbox {
    state: Arc<Mutex<UiInputMailboxState>>,
}

#[derive(Debug, Default)]
struct UiInputMailboxState {
    pending: VecDeque<SessionInteractionCandidate>,
    dropped: u32,
}

impl UiInputMailbox {
    /// Creates an empty per-session input mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one candidate or records an overflow when the queue is full.
    ///
    /// The return value tells a native boundary whether this particular
    /// candidate entered the queue. Existing callers that only need bounded
    /// best-effort delivery can keep using [`push`](Self::push); a boundary
    /// that has to acknowledge an action to another API can use this result
    /// without inspecting the queue or its other contents.
    pub fn try_push(&self, candidate: impl Into<SessionInteractionCandidate>) -> bool {
        let candidate = candidate.into();
        let state = &mut *lock(&self.state);
        if state.pending.len() == UI_INPUT_QUEUE_CAPACITY {
            state.dropped = state.dropped.saturating_add(1);
            false
        } else {
            state.pending.push_back(candidate);
            true
        }
    }

    /// Adds one candidate or records an overflow when the queue is full.
    ///
    /// This is the best-effort form used by local native input. The dropped
    /// count remains available to the granted consumer on the next drain.
    pub fn push(&self, candidate: impl Into<SessionInteractionCandidate>) {
        let _ = self.try_push(candidate);
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
    candidates: Vec<SessionInteractionCandidate>,
    dropped: u32,
}

impl UiInputBatch {
    /// Returns candidates in their original native input order.
    #[must_use]
    pub fn into_candidates(self) -> Vec<SessionInteractionCandidate> {
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

#[cfg(test)]
mod tests {
    use anodrel_menu::{ContextMenuRevision, MenuActionId, MenuRevision};
    use anodrel_ui::{ElementId, UiEvent};

    use super::{UI_INPUT_QUEUE_CAPACITY, UiInputMailbox};
    use crate::{
        ContextMenuInputCandidate, MenuInputCandidate, SessionInteractionCandidate,
        UiDocumentRevision, UiInputCandidate,
    };

    fn candidate() -> UiInputCandidate {
        UiInputCandidate::new(
            UiDocumentRevision::INITIAL
                .next()
                .expect("the first revision exists"),
            UiEvent::ActionInvoked(ElementId::new("action").expect("fixed ID is valid")),
        )
    }

    #[test]
    fn try_push_reports_admission_without_changing_overflow_accounting() {
        let mailbox = UiInputMailbox::new();
        for _ in 0..UI_INPUT_QUEUE_CAPACITY {
            assert!(mailbox.try_push(candidate()));
        }
        assert!(!mailbox.try_push(candidate()));

        let batch = mailbox.drain();
        assert_eq!(batch.dropped(), 1);
        assert_eq!(batch.into_candidates().len(), UI_INPUT_QUEUE_CAPACITY);
    }

    #[test]
    fn preserves_document_and_menu_candidates_in_one_ordered_queue() {
        let mailbox = UiInputMailbox::new();
        let document_revision = UiDocumentRevision::INITIAL
            .next()
            .expect("the first document revision exists");
        let menu_revision = MenuRevision::INITIAL
            .next()
            .expect("the first menu revision exists");
        let context_menu_revision = ContextMenuRevision::INITIAL
            .next()
            .expect("the first context-menu revision exists");
        mailbox.push(UiInputCandidate::new(
            document_revision,
            UiEvent::ActionInvoked(ElementId::new("document.action").expect("test ID is valid")),
        ));
        mailbox.push(MenuInputCandidate::new(
            menu_revision,
            MenuActionId::new("menu.action").expect("test ID is valid"),
        ));
        mailbox.push(ContextMenuInputCandidate::new(
            context_menu_revision,
            MenuActionId::new("context-menu.action").expect("test ID is valid"),
        ));

        let candidates = mailbox.drain().into_candidates();
        assert_eq!(candidates.len(), 3);
        let SessionInteractionCandidate::Ui(document) = &candidates[0] else {
            panic!("the first candidate is a document action");
        };
        assert_eq!(
            document.clone().into_parts().0,
            document_revision,
            "document action order changed"
        );
        let SessionInteractionCandidate::Menu(menu) = &candidates[1] else {
            panic!("the second candidate is a menu action");
        };
        let (revision, action) = menu.clone().into_parts();
        assert_eq!(revision, menu_revision);
        assert_eq!(action.as_str(), "menu.action");
        let SessionInteractionCandidate::ContextMenu(context_menu) = &candidates[2] else {
            panic!("the third candidate is a context-menu action");
        };
        let (revision, action) = context_menu.clone().into_parts();
        assert_eq!(revision, context_menu_revision);
        assert_eq!(action.as_str(), "context-menu.action");
    }
}
