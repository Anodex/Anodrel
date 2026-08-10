# Decision 0065: A crash record is host-only, payload-free, and covers the easy case honestly

**Status:** Accepted

**Date:** 2026-08-10

## Context

The Windows window procedure is `extern "system"` and does not unwind, so an
escaping panic aborts the process and runs no destructor. The threat model
records what that would cost: a verified product child left running with no
host, and a notification-area entry on screen with nothing behind it. Panic
containment fixed that — a contained panic ends the message loop and the
ordinary drop paths clean up.

Containment leaves no trace. `ROADMAP.md` has always kept crash *reporting* —
any persisted or transmitted record of a failure — as separate work, and this is
that work. An operator who finds the host gone currently cannot tell a clean
exit from a defect that was handled.

Three questions had to be answered before writing anything to disk.

**Where does it go?** The existing directory layout is per-application:
`Anodrel\Applications\<applicationId>\logs`. A host crash is not an
application's, and the host does not always have an application identity at all.

**What may it contain?** The panic payload is the obvious thing to record and
the one thing that cannot be recorded. A payload is whatever a failing
expression happened to hold: a path, a pipe name, a token, a fragment of
application content. `contain_panic` already drops it.

**How much does it claim to cover?** A component named for crashes invites the
assumption that it catches crashes. Almost none of the ways a native process
dies involve an unwinding Rust panic.

## Decision

**Records go to a host-owned location.** `anodrel-paths` gains a host layout
under `Anodrel\Host`, a sibling of `Applications`, with a `logs` location. It
performs no filesystem mutation, matching the existing layout builder. Filing a
host defect under whichever application happened to be loaded would both
misattribute it and leak one application's presence into another's directory.

**A record carries a closed catalogue and nothing else.** Site, surface, host
version, sequence, and a format tag. No panic payload, no path, no native
status, no identifier, no wall-clock time. The reporter is never handed the
payload, so no future edit can start including it by accident.

Leaving out the clock costs real diagnostic value: records cannot be ordered
against anything outside the process that wrote them. It is still right for v1.
A timestamp is the first field in this format that would describe the person at
the machine rather than the defect, and the sequence plus file name already
order records where it matters. Adding one later is a format-version decision,
made deliberately, not a field someone slips in.

**The scope is stated as a limitation, not implied by the name.** This records a
contained Rust panic. It does not catch access violations, stack overflow,
`abort`, a panic while panicking, a hang, or a child's crash.
`docs/CRASH_REPORTS.md` says so in a table, because the failure mode of a crash
reporter is a reader believing silence means health.

**There is no protocol surface, and this is load-bearing.** No operation reads,
writes, or observes a record. The in-memory ledger is readable by an
authenticated session holding `diagnostics.read`; a crash record is not readable
at all. The two boundaries point in opposite directions and must not be merged —
a ledger event about a crash would put host defect information behind a grant an
application can hold. A test asserts that no protocol operation names a crash
record.

**Reporting is best effort and silent.** The host is already shutting down. A
reporter that panics, blocks, or complains would turn a handled defect into an
unhandled one, so every failure resolves to a bounded category and shutdown
proceeds regardless.

## Consequences

The host gains its first write to disk that is not application state, and its
first location outside the per-application namespace. Both are documented as
layout extensions rather than new capabilities: no protocol operation, grant, or
application-visible behaviour changes.

Retention is bounded at 8 records and eviction is by sequence, so a host that
panics repeatedly cannot fill a disk. A record is capped at 512 bytes and the
writer refuses an oversized one rather than truncating, because a truncated
record is one whose meaning cannot be trusted.

The honest scope means this will not explain the failures most likely to matter
later. A structured-exception handler runs inside a damaged process where
allocation, locking, and reentrancy are unsafe; it is a different discipline
with different rules and deserves its own decision rather than being grown out
of this one.

## Alternatives considered

**Add a crash event to the in-memory ledger.** Cheapest by far, and wrong: the
ledger is readable through `diagnostics.read`, so a host defect would become
information an application can request. It also does not survive the process,
which is the entire point.

**Write into the application's `logs` directory.** Already available, no new
layout. Misattributes the defect and puts one application's evidence where
another application's host will look.

**Record the panic payload, redacted.** Every redaction scheme is a guess about
what a value contains. The payload's whole hazard is that its contents are
unknown at compile time. Dropping it is the only rule that holds.

**Include a timestamp.** More useful, and the reason it is deferred rather than
refused: it is the first field describing the person rather than the defect.
Worth revisiting with the packaging decision, when there is a shipped product
whose support story needs it.
