# Decision 0217: Initial-install failures use fixed safe statuses

**Status:** Accepted

**Date:** 2026-09-08

## Context

The default installer starts a separate elevated `install` child. Its parent
previously retained only zero or nonzero, making every elevated failure look
identical even when the installer had already identified a safe transaction
stage. Relaying arbitrary child console output, raw Windows errors, paths, or
policy state would turn a narrow UAC boundary into a diagnostics channel and
could incorrectly imply a completed installation.

## Decision

The fixed elevated `install` command returns one small conventional exit value
for each existing path-free installation error class. The no-argument parent
may map only those exact values to existing safe stage messages. It maps every
unknown nonzero value to the generic failure message, never parses child output,
and does not display the numeric code.

Zero remains the only outcome permitted to enter the existing postcondition
proof. A nonzero status is not policy proof, installation success, or a report
of what changed on the machine.

## Consequences

- A person can distinguish fixed release, policy, preparation, promotion,
  publication, Start-menu, and Apps & features failures without machine detail.
- The UAC boundary continues to expose no caller-selected command, output, or
  diagnostic channel.
- Tests must prove that unknown outcomes stay generic and that a nonzero child
  cannot enter the policy proof.

## Revisit conditions

Revisit for a dedicated installer window, structured local diagnostics,
localization, repair, managed deployment, restart coordination, another scope,
or another operating system.
