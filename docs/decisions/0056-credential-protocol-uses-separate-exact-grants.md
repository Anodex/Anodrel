# Decision 0056: Credential protocol uses separate exact grants

**Status:** Accepted

**Date:** 2026-08-01

## Context

The Windows Credential Manager adapter and identity-bound service seam can
already store arbitrary bounded bytes under one host-derived application
namespace. A public operation must not turn that into target selection,
enumeration, metadata discovery, shared credentials, or ambient authority.

## Decision

Protocol 1.12 adds three immediate operations: `credential.read`,
`credential.write`, and `credential.delete`. Each carries one exact bounded
credential name; write additionally carries one canonical lowercase hexadecimal
secret. They require distinct host-issued grants with the same names.

Read returns either a secret for that exact name or `not_found`; delete returns
only `deleted` or `not_found`; write returns only `written`. Names, secrets,
targets, application identity, Windows status, persistence metadata, timestamps,
and any other credential information never enter logs, diagnostics, events,
errors, or capability discovery beyond the grant labels themselves.

The core receives a `CredentialService` explicitly bound to the host-validated
application identity. The renderer, request, bootstrap invitation, and pipe
session never select an identity or Credential Manager target. An unavailable,
access-denied, malformed persisted-secret, or invalid payload returns only a
safe structured category.

## Consequences

Positive:

- applications can use OS-backed credentials through a small documented SDK;
- read, write, and delete authority remain independently grantable and
  revocable; and
- arbitrary binary secrets retain an owned canonical representation.

Tradeoffs:

- secret text is present in an authenticated response by design and callers
  must still handle it as secret material;
- no consent prompt, sharing, enumeration, rotation, watching, or cancellation
  of started Credential Manager work is introduced; and
- installed-policy grants and a direct native integration diagnostic remain
  required before this becomes a product session capability.

## Revisit conditions

Revisit for user consent, hardware-backed keys, non-Windows keychain adapters,
secret rotation, credential metadata, binary protocol envelopes, cross-device
sync, or long-running cancellable credential operations.
