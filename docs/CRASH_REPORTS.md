# Anodrel Crash Records

**Status:** Host-only bounded record of a contained panic. Not an application
capability, not a telemetry pipeline, and not a general crash handler.

## Purpose and boundary

Decision 0060 gave the Windows host a single lifetime owner, and the panic
containment recorded in `docs/THREAT_MODEL.md` stops a host defect from
aborting the process and stranding a verified product child. Containment
shuts the host down in an orderly way — and then leaves nothing behind. An
operator who finds the host gone has no way to tell a clean exit from a
contained defect.

A crash record is that missing evidence, and nothing more. It is written by the
host, to the host's own location, for the person sitting at the machine.

It is **not**:

- an application API — no protocol operation reads, writes, or observes a
  record, and none is planned;
- a transmitted report — nothing leaves the machine, ever;
- a general crash handler — see [What it does not catch](#what-it-does-not-catch);
- a durable audit record — retention is bounded and records are disposable.

The in-memory ledger of `docs/LOGGING.md` remains a separate thing: it is
readable by an authenticated session holding `diagnostics.read`, and it never
gains a crash event. A crash record is written to disk and is not readable
through the protocol at all. The two boundaries are opposites and must not be
merged.

## What it does not catch

This records a **contained Rust panic** — one that
`std::panic::catch_unwind` returned from at a boundary the host chose. That is
the whole of it.

It does not catch, and this is not a gap to be quietly filled later:

| Failure | Why not |
| --- | --- |
| Access violation, illegal instruction, other SEH exception | Needs a structured-exception or vectored handler running in a damaged process. A different discipline with different rules; see below. |
| Stack overflow | The guard page is already gone; no ordinary allocation is safe. |
| `abort`, including a panic while panicking | Terminates without unwinding. Nothing runs. |
| A hang, deadlock, or livelock | Nothing panics; the process is alive and wrong. |
| A crash in a child process | The child's own concern. The host records only that a session ended. |

Anything in that table needs a handler that runs inside a broken process, where
allocation, locking, and reentrancy are all unsafe. This crate deliberately does
none of that. A contained panic is the easy case — the stack has already
unwound and the process is healthy again — and the easy case is worth having on
its own.

## What a record contains

Exactly these fields, and no others:

| Field | Meaning | Source |
| --- | --- | --- |
| `format` | Always `anodrel.crash.v1`. | Constant. |
| `site` | Where containment happened, from the closed catalogue below. | The reporting call site. |
| `surface` | What kind of window was being served, from a closed catalogue. | The host's own view registry. |
| `hostVersion` | The host crate's compile-time version. | Constant. |
| `sequence` | Process-local order, starting at 1. | The reporter. |

The closed site catalogue:

| Site | Meaning |
| --- | --- |
| `window-procedure` | A panic escaped one window-message dispatch. |

The closed surface catalogue:

| Surface | Meaning |
| --- | --- |
| `startup-lab` | The branded startup surface. |
| `document` | A host-composed document window. |
| `ui-lab` | The owned UI foundation lab. |
| `ui-session` | A development UI session view. |
| `unknown` | No view was registered for the window. |

### What a record must never contain

The panic payload is **discarded, not inspected**. A panic message can carry any
value a failing expression happened to hold — a path, a pipe name, a fragment of
application content, a token. Nothing derived from a payload may reach a record.
`contain_panic` drops it, and the reporter is never given it.

A record also carries no absolute path, native status code, thread or process
identifier, user or machine name, wall-clock time, application content, package
manifest text, capability context, invitation, or credential.

The absence of a clock is deliberate and costs something real: two records
cannot be ordered against anything outside the process that wrote them. The
`sequence` field orders records within one process, and the file name orders
them within the directory. A wall-clock reading would be more useful and would
also be the first field in this format that describes the person rather than the
defect, so v1 does without and says so.

## Location, format, and retention

Records go to the host's own diagnostics location, not to an application's:

~~~text
%LOCALAPPDATA%\Anodrel\Host\logs\
~~~

A crash in the host is the host's, and filing it under whichever application
happened to be loaded would misattribute it and leak one application's presence
into another's directory. `docs/PATHS.md` records this location as a compatible
extension to layout v1.

Each record is one file named `crash-<sequence>.anodrel.v1`, holding one record
serialized as strict `field=value` lines in the fixed order above, ASCII only,
newline separated. The format is a local host format, not a protocol; it has no
reader in this repository beyond its own tests.

At most **8** records are retained. Writing the ninth removes the oldest by its
sequence. A record is at most 512 bytes; the writer refuses anything larger
rather than truncating, because a truncated record is a record whose meaning
cannot be trusted.

## Failure behaviour

Reporting is best effort and silent. The host is already shutting down after a
contained panic, and a reporter that panics, blocks, or complains would turn a
handled defect into an unhandled one. Every failure — no known folder, a
directory that cannot be created, a disk that is full, a file that cannot be
opened — resolves to a bounded category carrying no path and no native status,
and the shutdown proceeds either way.

The categories are `LocationUnavailable`, `WriteFailed`, `RecordTooLarge`, and
`RecordMalformed`. The last covers a field the line format cannot carry — only
the host version can reach it, since every other field is a catalogue value or a
counter, and it exists so a version that stopped being a compile-time constant
could not forge a second field with a newline.

The reporter performs no retry, no fallback location, and no user-visible
message. Nothing about a failed report reaches an application, the ledger, or a
rendered surface.

## Verification

Portable unit tests cover the closed catalogues, the exact serialization,
sequence assignment, the size bound, and that failure categories carry no
detail. Adapter tests write into a temporary directory and cover creation,
retention, and eviction order. A host test proves that a contained panic
produces exactly one record and that the message loop still ends.

A separate test asserts that no protocol operation names a crash record, which
is the invariant most likely to be broken by someone adding a convenience later.

Manual check on Windows:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --crash-report-selftest
~~~

That route writes one record from the ordinary reporting path and prints only
whether it succeeded, then exits. Inspect the directory above; the file is
plain text. Delete the directory to reset.

## Compatibility

`anodrel.crash.v1` is a closed format. Adding a site or surface is additive.
Adding a field, a clock, any caller-supplied text, a protocol reader, a
transmitted report, or a handler for the failures listed under
[What it does not catch](#what-it-does-not-catch) requires a documented service
contract, a capability decision, a threat-model update, and a new format
version. Decision 0065 records the reasoning.
