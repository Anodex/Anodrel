# Application state storage v1

**Status:** Portable value and host-service foundation. No native adapter or
public protocol operation is available yet.

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
The application never supplies a location. A Windows adapter will retain a
validated current-user file object while reading or replacing the state and
will keep a bounded prior committed snapshot as a recovery candidate. It will
write a complete new snapshot to a host-chosen temporary object, flush it, then
perform one direct Windows replacement operation. A failed or interrupted
write must leave either the prior complete snapshot or a complete new snapshot
available; it must never return a partial value.

Snapshot contents, absolute paths, temporary names, native status values, and
recovery details must not appear in protocol diagnostics or the typed host log.
The adapter must reject links, directories, and malformed storage records.

## Permissions and compatibility

This document intentionally defines no protocol operation or capability. A
future protocol must add independent host-issued read, replace, and clear
grants; document its exact payload and result forms; and add contract coverage
before any authenticated application can use storage. The protocol must not
accept an absolute or relative path.

The v1 snapshot limit, whole-value semantics, and one-snapshot namespace are
part of the compatibility contract. A key-value API, binary-transfer surface,
larger quota, cross-device sync, encryption policy, directory access, and
concurrent multi-process writer policy require separate decisions.

## Verification plan

The portable foundation tests absent versus empty state, the fixed size limit,
value redaction, and error categories. The Windows adapter will test
current-user identity isolation, atomic replacement, recovery from an
interrupted staging file, malformed-record rejection, link rejection, and that
no storage path or content appears in safe error output. Authenticated protocol
coverage will be added only with the later capability contract.

Decision 0051 records this boundary.
