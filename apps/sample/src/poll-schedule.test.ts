import assert from "node:assert/strict";
import test from "node:test";

import {
  FIRST_INTERVAL_MILLISECONDS,
  MAXIMUM_INTERVAL_MILLISECONDS,
  WAIT_BUDGET_MILLISECONDS,
  pollSchedule,
} from "./poll-schedule.js";

test("the first poll answers an immediate activation", () => {
  const [first] = pollSchedule();
  assert.equal(first, FIRST_INTERVAL_MILLISECONDS);
  assert.ok(FIRST_INTERVAL_MILLISECONDS < 50);
});

test("intervals never shrink and never exceed the cap", () => {
  let previous = 0;
  for (const interval of pollSchedule()) {
    assert.ok(interval >= previous, "the schedule must not shrink");
    assert.ok(interval <= MAXIMUM_INTERVAL_MILLISECONDS, "the schedule must not exceed its cap");
    previous = interval;
  }
  assert.equal(previous, MAXIMUM_INTERVAL_MILLISECONDS, "the schedule must reach its cap");
});

test("the whole wait stays within one interval of its budget", () => {
  // The loop stops as soon as the accumulated wait reaches the budget, so it
  // can overshoot by at most one capped interval.
  let total = 0;
  for (const interval of pollSchedule()) {
    total += interval;
  }
  assert.ok(total >= WAIT_BUDGET_MILLISECONDS, "the wait must cover its whole budget");
  assert.ok(
    total <= WAIT_BUDGET_MILLISECONDS + MAXIMUM_INTERVAL_MILLISECONDS,
    "the wait must not run long",
  );
});

test("backing off removes most of the idle round trips", () => {
  const polls = [...pollSchedule()].length;
  const fixedIntervalPolls = WAIT_BUDGET_MILLISECONDS / 100;

  assert.ok(
    polls * 5 < fixedIntervalPolls,
    `backing off saved too little: ${polls} polls against ${fixedIntervalPolls}`,
  );
});
