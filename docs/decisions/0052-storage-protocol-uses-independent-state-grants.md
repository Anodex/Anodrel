# Decision 0052: Storage protocol uses independent state grants

**Status:** Accepted

**Date:** 2026-08-01

## Context

The portable storage service and direct Windows adapter now hold one bounded,
host-derived application-state snapshot, but application code has no protocol
route to it. A single broad storage permission would make it difficult for a
machine policy to allow reading without replacement or let a recovery utility
clear state without disclosing it. The portable 256 KiB bound also cannot be
passed safely through the 64 KiB Wire 1.0 message envelope.

## Decision

Protocol 1.10 adds three exact operations:

- `storage.state.read` with exact `{}` payload and `storage.state.read` grant;
- `storage.state.replace` with exact `{ "snapshot": string }` payload and
  `storage.state.replace` grant; and
- `storage.state.clear` with exact `{}` payload and `storage.state.clear`
  grant.

The protocol snapshot field has a 24 KiB UTF-8 limit. This is a transport
limit, not a reduction of the portable host-store limit. A read returns either
`{ "status": "snapshot", "snapshot": string }` or
`{ "status": "absent" }`; replacement and clear return fixed accepted
statuses. The protocol accepts no path, key, filename, range, binary encoding,
directory, stream, or temporary-name field.

The core checks each capability immediately before calling its injected storage
service. A host that did not supply a storage service fails closed with
`storage.unavailable`. Stored invalid or oversized values return only their
stable safe storage categories, without state contents, paths, recovery source,
or native detail.

## Consequences

- machine policy can grant state actions with least privilege;
- SDK, mock, core, transport, and native host can use one versioned contract;
- the direct Windows recovery adapter remains hidden behind the portable
  service interface; and
- snapshots larger than 24 KiB remain host-internal until a separate bounded
  transfer design exists.

## Revisit conditions

Revisit before adding keys, binary values, partial updates, compare-and-swap,
concurrent writers, subscriptions, paths, quota reporting, encryption policy,
or a larger/chunked protocol transfer.
