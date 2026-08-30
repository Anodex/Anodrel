# Application state storage v1

**Status:** Implemented portable value, direct Windows host service, direct
Linux host adapter, Protocol 1.10 development-session path, and
identity-bound registered Windows session composition.

## Purpose and boundary

Applications need durable state, but a general filesystem bridge would let a
renderer choose paths, enumerate a profile, or accidentally couple two
applications. The first Anodrel storage surface is therefore one opaque state
snapshot per host-validated application identity. The application owns the
snapshot's schema and migration; the platform owns location, isolation, size,
atomic replacement, and recovery.

The storage surface does not expose a path, directory, filename, handle,
stream, watcher, database, or file enumeration. It is not a replacement for a
future scoped document-access feature.

## Planned value contract

`anodrel-storage` defines the portable storage value as an arbitrary UTF-8 byte
sequence of at most **256 KiB**. It may be empty. An absent snapshot is distinct
from an empty snapshot. The host-service interface exposes only these
operations:

| Operation | Input | Result |
| --- | --- | --- |
| Read | none | absent, or the complete saved snapshot |
| Replace | one complete bounded snapshot | success only after the host accepts the replacement |
| Clear | none | success only after the host accepts removal of the saved snapshot |

There is no partial read, append, seek, key, path, directory selector, or
application-controlled temporary name. The API makes whole-state replacement
explicit so the recovery contract remains small and testable.

## Isolation and recovery

The host derives the storage location exclusively from the already validated
application identity and the host-owned `data` location from `docs/PATHS.md`.
The application never supplies a location. The direct Windows adapter creates
only the derived directory tree, rejects reparse points for each derived
directory and state file, and keeps a bounded prior committed snapshot as a
recovery candidate. It writes a complete new snapshot to a fixed host-chosen
staging file and flushes it before moving the prior state to the backup and the
staged state into place through direct Windows rename operations.

The direct Linux adapter derives the same layout from the effective account
root and opens the pre-existing account home component by component. It creates
only the fixed Anodrel subtree with private 0700 directories and opens the
three fixed state names through the resulting `data` directory descriptor.
Every state object must be an effective-account-owned single-link regular file
with private 0600 permissions; a symbolic link, directory, hard link, malformed
component, or unexpected ownership fails closed. The adapter writes and syncs a
new staging file, moves the prior state to the fixed backup, moves staging into
the current location, and syncs the data directory after each rename.

On both systems, a failed or interrupted replacement leaves either the prior
complete snapshot or a complete new snapshot available; it never returns a
partial value. Reading prefers current state, then one valid backup. Staging is
never readable state. Clear removes only the fixed state files. Multiple host
processes writing one identity are not a v1 policy and must not be inferred
from the per-service operation lock.

Snapshot contents, absolute paths, temporary names, native status values, and
recovery details must not appear in protocol diagnostics or the typed host log.
The adapter must reject links, directories, and malformed storage records.

## Permissions and compatibility

Protocol 1.10 reserves independent host-issued `storage.state.read`,
`storage.state.replace`, and `storage.state.clear` grants. Its exact operations
are `storage.state.read` (`{}`), `storage.state.replace`
(`{ "snapshot": string }`), and `storage.state.clear` (`{}`). Read returns
either `{ "status": "snapshot", "snapshot": string }` or
`{ "status": "absent" }`; replace and clear return fixed accepted statuses.
The protocol snapshot is limited to **24 KiB**, independently of the portable
256 KiB store limit, so it remains safe inside Wire 1.0. No protocol form
accepts an absolute or relative path.

The core must check the matching host-issued grant immediately before calling
its injected storage service. A host that has not explicitly supplied one
returns only `storage.unavailable`. Stored invalid and oversized values map to
their stable safe error categories without state contents, paths, recovery
source, or native details.

Registered Windows sessions attach this service only after their installed
record validates and derive its location from that record's identity. Version
1.2 records may select the three exact storage grants; version 1.1 records
cannot select them. A signed provisioned product launch remains separate from
this composition boundary.

The v1 snapshot limit, whole-value semantics, and one-snapshot namespace are
part of the compatibility contract. A key-value API, binary-transfer surface,
larger quota, cross-device sync, encryption policy, directory access, and
concurrent multi-process writer policy require separate decisions.

## Verification plan

The portable foundation tests absent versus empty state, the fixed size limit,
value redaction, and error categories. The Windows adapter tests whole-value
replacement, recovery from an interrupted staging file, and path redaction.
Its direct file boundary rejects directories and reparse points. The Linux
adapter tests whole-value replacement, recovery, private modes, and rejected
symbolic-link or hard-link state files through a real Linux filesystem. Shared
protocol contract tests cover the exact read, replace, and clear messages,
their independent grants, and the protocol request bound. The Windows
development UI-session diagnostic replaces, reads, and clears one test snapshot
through the authenticated pipe before completing its semantic UI round trip.

Decisions 0051, 0052, and 0125 record these boundaries.
