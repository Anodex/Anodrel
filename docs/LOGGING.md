# Anodrel Host Diagnostic Log

**Status:** Bounded closed catalogue with a Protocol 1.11 authenticated read.

## Purpose

The first Anodrel log is a bounded, in-memory record of safe host startup
events. It exists so the Startup Lab can demonstrate an actual **Open Logs**
action without creating a general-purpose sink for secrets, paths, application
text, or native failures.

It is a host diagnostic, not an application logging API, telemetry pipeline,
crash reporter, or durable audit record.

## Contract

`anodrel-diagnostics` owns a `LogBook` with a fixed maximum of 64 entries. A
record has exactly four display-safe fields:

| Field | Meaning | Source |
| --- | --- | --- |
| `sequence` | Monotonic order within this process. | Assigned by `LogBook`. |
| `level` | Current severity (`info`). | The typed event declaration. |
| `component` | Fixed host component identifier. | The typed event declaration. |
| `event` | Fixed human-readable event text. | The typed event declaration. |

Entries are a closed enum, not caller-provided strings. The current events are:

| Component | Event |
| --- | --- |
| `package` | Application package verification completed. |
| `core` | Internal `platform.health` check completed. |
| `transport` | Private named-pipe loopback completed. |
| `launch` | Verified launch preflight completed. |
| `launch` | Verified launch preflight could not run. |
| `host` | Startup Lab launch authorized. |

When the capacity is reached, the oldest entry is discarded. The log reads no
clock and records no duration, absolute path, request payload, manifest text,
pipe name, invitation, capability context, token, credential, raw native error,
or application content.

### Why the launch events state execution, not outcome

The two `launch` events say whether this host completed its verification-only
preflight. Neither says what the preflight concluded. That distinction is
deliberate: the answer is machine state about a *different* application's
provisioning, and this ledger is readable by an authenticated session that holds
`diagnostics.read`. A reader learning "the host checked" is harmless; a reader
learning "another application is provisioned on this machine" would be a small
but real cross-application disclosure.

The pair still carries the one thing an operator cannot see on the surface. A
declined preflight and an absent preflight both leave the launch tile planned,
so the log is the only place to tell "checked and declined" from "never
checked".

## Display and lifecycle

The Windows host creates this log only after all Startup Lab preflight checks
have passed. Exactly one of the two `launch` events is recorded per surface, and
the same resolved preflight outcome selects both that event and the launch
tile's availability, so the log and the surface cannot disagree. The linked
**Open Logs** tile opens a native document window from the log snapshot. The document can show only the four fields above and
cannot navigate, export, write a file, or contact another process.

The log is dropped with the host process. It is not written to disk, included in
crash output, exported, or writable by an application. Protocol 1.11 can expose
the exact retained closed records to an authenticated session through
`diagnostics.entries.read` only when its host policy grants `diagnostics.read`.
The read accepts exactly `{}` and returns at most 64 records with only the four
fields above. It accepts no filter, cursor, time, path, arbitrary text,
subscription, or acknowledgement. A host without an explicitly supplied log
service returns only `diagnostics.unavailable`.

## Compatibility

The catalogue remains a closed Rust contract; the Protocol 1.11 reader is a
strict projection of it, not a general SDK logging API. Adding a new typed event
is additive only when its component and text are reviewed as display-safe.
Accepting dynamic fields, application-originated events, persistence, export,
or a broader reader requires a documented service contract, a capability
decision, threat-model update, and compatibility tests.

## Verification

Unit tests prove that the ledger bounds itself, preserves the order of retained
events, assigns process-local sequence numbers, only exposes the closed event
catalogue, and that the two `launch` events describe host execution rather than
machine state. Protocol contract tests verify its exact payload, fixed record
shape, independent grant check, and unavailable service behavior. Host tests
prove that the linked log action produces a document and that no document action
carries a filesystem path.
