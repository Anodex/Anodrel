# Decision 0180: Default installer entry composes fixed initial-install stages

**Status:** Accepted

**Date:** 2026-09-02

## Context

The owned initial-install preflight, native consent, UAC handoff, and
postcondition proof are independently safe but do not make an installer a
person can start normally. A broad command parser or a generic coordinator
would undermine their fixed-purpose contracts. Direct `install` remains useful
for an explicit elevated operator route and should not be replaced.

## Decision

Make the no-argument installer entry compose only the opaque first-install
stages in their fixed order: signed missing-policy preflight, native consent,
fixed UAC `install`, non-UI-thread process wait, and postcondition proof. It
prints only a closed console outcome. A decline is ordinary no-change success;
all other failed stages return their existing safe failures. The route refuses
an existing selected policy and never interprets the current installer image as
an update.

The route accepts no input. Named commands retain their current fixed shapes
and elevation rules. It shows no progress, completion dialog, restart prompt,
or custom installer window.

## Consequences

- A person can start a signed initial installation without finding an elevated
  shell or supplying a command.
- The elevated child remains the existing fixed `install` command and repeats
  all release and policy gates.
- The first interactive surface stays small enough to prove in a signed
  development fixture before a branded installer shell is considered.

## Revisit conditions

Revisit for a full owned installer window, progress, repair, update choice,
restart coordination, managed deployment, localization, another installation
scope, a signed positive fixture run, or another platform.
