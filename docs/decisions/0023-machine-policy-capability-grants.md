# Decision 0023: Bind session capability grants to the installed record

**Status:** Accepted

**Date:** 2026-07-31

## Decision

Installed application records add version 1.1. It requires a `capabilities`
array containing exact supported grant strings. Version 1.0 remains valid and
grants no capabilities. Unknown, duplicate, malformed, or unsupported grants
fail closed.

The session-policy module converts only a validated installed record
into the existing host policy. Packages, bootstrap data, pipe clients, protocol
messages, and UI cannot select or elevate these grants. Supported grants now
include separate `clipboard.read` and `clipboard.write` values; adding either
does not grant the other.

## Consequences

- machine policy becomes the durable source for authenticated child-session
  grants;
- old records remain safe during migration because they authorize no service;
- this adds no public protocol operation or live launch integration.

## Revisit conditions

Revisit for user consent, per-operation grant parameters, revocation, public
session lifecycle, or an additional protocol capability.
