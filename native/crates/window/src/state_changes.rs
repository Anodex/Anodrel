//! Coalesced, pull-only presentation changes for one session window.
//!
//! The host UI thread records only a changed portable state. A protocol worker
//! consumes at most that one latest value; there is no timestamp, history,
//! waiter, callback, subscription, or unbounded queue.

use std::{
    fmt,
    sync::{Arc, Mutex},
};

use crate::WindowState;

/// The portable service boundary for one coalesced state-change read.
pub trait WindowStateChangesService: std::fmt::Debug + Send {
    /// Consumes the latest observed change, if this session has one.
    ///
    /// A `None` value says only that no unread change is retained. It does not
    /// report the current state or any native timing or window detail.
    fn read_change(&self) -> Result<Option<WindowState>, WindowStateChangesServiceError>;
}

/// A safe failure category for a coalesced state-change service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowStateChangesServiceError {
    /// This session has no available host-owned state-change surface.
    Unavailable,
}

impl fmt::Display for WindowStateChangesServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => {
                formatter.write_str("no session window state changes are available")
            }
        }
    }
}

impl std::error::Error for WindowStateChangesServiceError {}

/// One bounded coalescing mailbox from a session UI thread to its worker.
#[derive(Clone, Debug, Default)]
pub struct WindowStateChangesMailbox {
    state: Arc<Mutex<StateChanges>>,
}

#[derive(Debug, Default)]
struct StateChanges {
    baseline: Option<WindowState>,
    unread: Option<WindowState>,
}

impl WindowStateChangesMailbox {
    /// Creates an empty, per-session state-change mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a UI-thread observation of this session window's state.
    ///
    /// The first observation establishes a baseline. Later distinct values
    /// replace any unread value, so rapid native transitions cannot retain a
    /// history or grow memory use.
    pub fn record_state(&self, state: WindowState) {
        let mut changes = lock(&self.state);
        if changes.baseline.is_some_and(|previous| previous == state) {
            return;
        }
        if changes.baseline.is_some() {
            changes.unread = Some(state);
        }
        changes.baseline = Some(state);
    }
}

impl WindowStateChangesService for WindowStateChangesMailbox {
    fn read_change(&self) -> Result<Option<WindowState>, WindowStateChangesServiceError> {
        Ok(lock(&self.state).unread.take())
    }
}

fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::{WindowStateChangesMailbox, WindowStateChangesService};
    use crate::WindowState;

    #[test]
    fn first_observation_establishes_a_baseline_without_a_change() {
        let mailbox = WindowStateChangesMailbox::new();

        mailbox.record_state(WindowState::Restored);

        assert_eq!(mailbox.read_change(), Ok(None));
    }

    #[test]
    fn a_distinct_observation_is_consumed_once() {
        let mailbox = WindowStateChangesMailbox::new();
        mailbox.record_state(WindowState::Restored);
        mailbox.record_state(WindowState::Maximized);

        assert_eq!(mailbox.read_change(), Ok(Some(WindowState::Maximized)));
        assert_eq!(mailbox.read_change(), Ok(None));
    }

    #[test]
    fn rapid_changes_coalesce_to_only_the_latest_state() {
        let mailbox = WindowStateChangesMailbox::new();
        mailbox.record_state(WindowState::Restored);
        mailbox.record_state(WindowState::Maximized);
        mailbox.record_state(WindowState::Minimized);
        mailbox.record_state(WindowState::Restored);

        assert_eq!(mailbox.read_change(), Ok(Some(WindowState::Restored)));
        assert_eq!(mailbox.read_change(), Ok(None));
    }

    #[test]
    fn a_repeated_state_does_not_create_a_change() {
        let mailbox = WindowStateChangesMailbox::new();
        mailbox.record_state(WindowState::Restored);
        mailbox.record_state(WindowState::Restored);

        assert_eq!(mailbox.read_change(), Ok(None));
    }
}
