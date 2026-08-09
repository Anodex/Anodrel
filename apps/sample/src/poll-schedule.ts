/**
 * The bounded backoff schedule used while waiting on a person.
 *
 * The development diagnostics deliver a document and then wait for someone to
 * activate the rendered action. Every poll is a real round trip: it wakes the
 * host's pipe worker, drains a mailbox, and encodes a response. A fixed short
 * interval spends that cost at a constant rate for as long as the window is
 * open, which is the wrong shape for work paced by a human.
 *
 * The schedule starts fast, so a click in the first moment is answered almost
 * immediately, then widens to a cap. It mirrors the schedule the native product
 * fixture uses, so both clients pace an idle window the same way.
 */

/** The first and shortest interval, covering an immediate activation. */
export const FIRST_INTERVAL_MILLISECONDS = 25;

/**
 * The longest interval the schedule ever waits.
 *
 * One second is the most a person should wait for a window to respond to their
 * own click, so this bounds the worst-case latency the backoff introduces.
 */
export const MAXIMUM_INTERVAL_MILLISECONDS = 1_000;

/**
 * The total time a client is willing to wait for one semantic action.
 *
 * Two minutes leaves room to inspect, scroll, or dismiss one host-owned dialog
 * without creating an unbounded background event read.
 */
export const WAIT_BUDGET_MILLISECONDS = 120_000;

/** How much each interval grows, kept as a rational so the sequence is exact. */
const GROWTH_NUMERATOR = 3;
const GROWTH_DENOMINATOR = 2;

/**
 * Yields a bounded, non-decreasing sequence of poll intervals in milliseconds.
 *
 * Iteration ends once the accumulated wait reaches the budget, so a caller can
 * loop over it and treat exhaustion as the timeout.
 */
export function* pollSchedule(): Generator<number, void, void> {
  let next = FIRST_INTERVAL_MILLISECONDS;
  let elapsed = 0;

  while (elapsed < WAIT_BUDGET_MILLISECONDS) {
    yield next;
    elapsed += next;
    next = Math.min(
      Math.floor((next * GROWTH_NUMERATOR) / GROWTH_DENOMINATOR),
      MAXIMUM_INTERVAL_MILLISECONDS,
    );
  }
}
