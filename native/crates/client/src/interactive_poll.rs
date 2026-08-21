//! Bounded pacing for one human-driven semantic action.
//!
//! Interactive Anodrel children poll the request/response event surface rather
//! than receiving a background callback. This schedule answers an immediate
//! action quickly, then backs off to avoid a constant stream of idle pipe
//! round trips while a person reads a window.

use std::time::Duration;

/// The first and shortest interval, covering an immediate activation.
const FIRST: Duration = Duration::from_millis(25);

/// The longest interval the schedule ever waits.
const CAP: Duration = Duration::from_secs(1);

/// The total time one development diagnostic waits for a person.
const BUDGET: Duration = Duration::from_secs(120);

/// A bounded, non-decreasing human-action polling schedule.
///
/// Iteration ends after it covers the fixed two-minute budget. The final sum
/// may exceed the budget by at most one capped interval, so callers can treat
/// exhaustion as a stable timeout without calculating time themselves.
#[derive(Clone, Copy, Debug)]
pub struct InteractivePollSchedule {
    next: Duration,
    elapsed: Duration,
}

impl InteractivePollSchedule {
    /// Creates the schedule for one complete interactive wait.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next: FIRST,
            elapsed: Duration::ZERO,
        }
    }
}

impl Default for InteractivePollSchedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for InteractivePollSchedule {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        if self.elapsed >= BUDGET {
            return None;
        }
        let interval = self.next;
        self.elapsed += interval;
        self.next = (interval * 3 / 2).min(CAP);
        Some(interval)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{BUDGET, CAP, FIRST, InteractivePollSchedule};

    #[test]
    fn the_first_poll_answers_an_immediate_activation() {
        assert_eq!(InteractivePollSchedule::new().next(), Some(FIRST));
        assert!(FIRST < Duration::from_millis(50));
    }

    #[test]
    fn intervals_never_shrink_or_exceed_the_cap() {
        let mut previous = Duration::ZERO;
        for interval in InteractivePollSchedule::new() {
            assert!(interval >= previous, "the schedule must not shrink");
            assert!(interval <= CAP, "the schedule must not exceed its cap");
            previous = interval;
        }
        assert_eq!(previous, CAP, "the schedule must reach its cap");
    }

    #[test]
    fn the_whole_wait_stays_within_one_interval_of_its_budget() {
        let total: Duration = InteractivePollSchedule::new().sum();
        assert!(total >= BUDGET, "the wait must cover its whole budget");
        assert!(total <= BUDGET + CAP, "the wait must not run long");
    }

    #[test]
    fn backing_off_removes_most_idle_round_trips() {
        let polls = InteractivePollSchedule::new().count();
        let fixed_interval_polls =
            (BUDGET.as_millis() / Duration::from_millis(100).as_millis()) as usize;

        assert!(
            polls * 5 < fixed_interval_polls,
            "backing off saved too little: {polls} polls against {fixed_interval_polls}"
        );
    }
}
