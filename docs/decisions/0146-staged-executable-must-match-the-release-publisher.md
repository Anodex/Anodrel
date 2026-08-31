# Decision 0146: Verify the staged executable before it can be promoted

**Status:** Accepted

**Date:** 2026-08-31

## Context

The installer self-signature proves the identity of the executable carrying the
release manifest and bundle. The staged package still contains a separate
native executable. Its raw bytes are bound to the manifest by SHA-256, but a
digest alone does not prove that the executable carries the publisher identity
the product host requires at launch.

Deferring this check until after a directory becomes a version directory or a
registry record is published would leave invalid installed content behind and
unnecessarily widen recovery logic.

## Decision

Add a first-party staged-signing gate. It can be reached only after the current
installer has passed its own Authenticode and embedded-publisher gate, the
release has been privately staged, and the staged package has passed the
installed-record validator. It asks the existing direct Windows Authenticode
adapter to verify the staged executable and requires its accepted opaque leaf
fingerprint to equal the publisher fingerprint in the same embedded manifest.

Only a result that passed all those gates may enter a later atomic promotion
transaction. The gate returns no path, certificate subject, fingerprint, native
trust status, or launch capability. It does not move a directory, write a
registry value, create trust, or start a process. Dropping its unpromoted stage
removes the private staging directory.

## Consequences

Positive:

- The installer image, manifest, bundle, staged executable, and eventual
  installed record share one publisher identity before machine policy changes.
- The existing narrow Authenticode adapter remains the sole certificate API.
- An unsigned or differently signed extracted executable fails before it can
  become a durable version directory.

Tradeoffs:

- A positive end-to-end automated test still requires an operator-provided
  resource-bearing installer and an executable signed by the same accepted
  development or production identity.
- The staged executable is checked again by the host's locked launch sequence;
  this installer-time check does not replace that later race-resistant check.

## Revisit conditions

Revisit for publisher-key rotation, a Windows package identity, or a signed
bundle format that replaces individual executable Authenticode validation.
