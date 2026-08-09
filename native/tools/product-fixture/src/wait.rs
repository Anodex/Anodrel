//! The bounded backoff schedule used while waiting on a person.
//!
//! The fixture delivers a document and then waits for someone to activate the
//! rendered action. Every poll is a real round trip: it wakes the host's pipe
//! worker, drains a mailbox, and encodes a response. A fixed short interval
//! would spend that cost at a constant rate for as long as the window is open,
//! which is the wrong shape for work paced by a human.
//!
//! The schedule starts fast, so a click in the first moment is answered almost
//! immediately, then widens to a cap. Over the same overall wait it makes about
//! an order of magnitude fewer round trips than a fixed interval.

use std::time::Duration;

/// The first and shortest interval, covering an immediate activation.
const FIRST: Duration = Duration::from_millis(25);

/// The longest interval the schedule ever waits.
///
/// One second is the most a person should wait for a window to respond to their
/// own click, so this bounds the worst-case latency the backoff can introduce.
const CAP: Duration = Duration::from_millis(1_000);

/// How much each interval grows, as a rational multiplier.
///
/// Integer arithmetic keeps the sequence exactly reproducible, which is what
/// lets the tests below assert on the total wait rather than approximate it.
const GROWTH_NUMERATOR: u32 = 3;
const GROWTH_DENOMINATOR: u32 = 2;

/// The total time the fixture is willing to wait for one semantic action.
///
/// Two minutes matches the other interactive diagnostics and keeps a forgotten
/// window from leaving a child running indefinitely.
const BUDGET: Duration = Duration::from_secs(120);

/// A bounded, non-decreasing sequence of poll intervals.
///
/// Iteration ends once the accumulated wait reaches the budget, so a caller can
/// simply loop over it and treat exhaustion as the timeout.
#[derive(Clone, Copy, Debug)]
pub struct PollSchedule {
    next: Duration,
    elapsed: Duration,
}

impl PollSchedule {
    /// Creates the schedule for one complete wait.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next: FIRST,
            elapsed: Duration::ZERO,
        }
    }
}

impl Iterator for PollSchedule {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        if self.elapsed >= BUDGET {
            return None;
        }
        let interval = self.next;
        self.elapsed += interval;
        self.next = (interval * GROWTH_NUMERATOR / GROWTH_DENOMINATOR).min(CAP);
        Some(interval)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{BUDGET, CAP, FIRST, PollSchedule};

    #[test]
    fn the_first_poll_answers_an_immediate_activation() {
        assert_eq!(PollSchedule::new().next(), Some(FIRST));
        assert!(FIRST < Duration::from_millis(50));
    }

    #[test]
    fn intervals_never_shrink_and_never_exceed_the_cap() {
        let mut previous = Duration::ZERO;
        for interval in PollSchedule::new() {
            assert!(interval >= previous, "the schedule must not shrink");
            assert!(interval <= CAP, "the schedule must not exceed its cap");
            previous = interval;
        }
        assert_eq!(previous, CAP, "the schedule must reach its cap");
    }

    #[test]
    fn the_whole_wait_stays_within_one_interval_of_its_budget() {
        // The loop stops as soon as the accumulated wait reaches the budget, so
        // it can overshoot by at most one capped interval.
        let total: Duration = PollSchedule::new().sum();
        assert!(total >= BUDGET, "the wait must cover its whole budget");
        assert!(total <= BUDGET + CAP, "the wait must not run long");
    }

    #[test]
    fn backing_off_removes_most_of_the_idle_round_trips() {
        let polls = PollSchedule::new().count();
        let fixed_interval_polls =
            (BUDGET.as_millis() / Duration::from_millis(100).as_millis()) as usize;

        assert!(
            polls * 5 < fixed_interval_polls,
            "backing off saved too little: {polls} polls against {fixed_interval_polls}"
        );
    }
}
