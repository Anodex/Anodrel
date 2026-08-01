# Application state storage v1

**Status:** Portable value, direct Windows host-service, and Protocol 1.10
contract foundation. Protocol wiring remains in progress.

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
staged state into place through direct Windows rename operations. A failed or
interrupted write leaves either the prior complete snapshot or a complete new
snapshot available; it never returns a partial value.

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

The v1 snapshot limit, whole-value semantics, and one-snapshot namespace are
part of the compatibility contract. A key-value API, binary-transfer surface,
larger quota, cross-device sync, encryption policy, directory access, and
concurrent multi-process writer policy require separate decisions.

## Verification plan

The portable foundation tests absent versus empty state, the fixed size limit,
value redaction, and error categories. The Windows adapter tests whole-value
replacement, recovery from an interrupted staging file, and path redaction.
Its direct file boundary rejects directories and reparse points. Shared
protocol contract coverage will be added with the implementation wiring.

Decisions 0051 and 0052 record these boundaries.
