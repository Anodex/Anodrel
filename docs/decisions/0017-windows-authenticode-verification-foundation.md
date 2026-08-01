# Decision 0017: Isolate Windows Authenticode verification from launch authority

**Status:** Accepted

**Date:** 2026-07-31

## Context

The controlled application package verifies a bounded text document but does
not authenticate its publisher. The existing private bootstrap can deliver a
secret to a child process, but deliberately accepts a developer-supplied
executable and establishes no executable identity.

The planned Launch Sample action needs an independently testable first-party
foundation for Windows executable signatures. Treating an executable's presence
inside an unpackaged directory, a manifest field from the same directory, or a
successful private-pipe connection as publisher identity would be unsafe.

## Decision

Create a Windows-only adapter that uses direct WinTrust and CryptoAPI calls to:

- verify a file's embedded Authenticode signature without trust-provider UI;
- obtain the leaf certificate only from a successful verification state; and
- return the leaf certificate SHA-256 fingerprint as a fixed-size value.

The adapter receives an internal canonical path and returns a small safe error
catalogue. It does not spawn a process, open a pipe, accept application input,
emit a protocol message, write a log, persist a certificate, render a
certificate subject, or expose an operating-system trust status.

The fingerprint is a comparison primitive, not an authorization decision. A
future installed publisher policy must provide the approved fingerprint and
bind it to a validated application ID outside the mutable package directory.
The complete future launch operation must also contain the executable, check
its digest, track its process lifetime, and use the private bootstrap only
after those checks.

## Consequences

Positive:

- executable-signature handling is isolated from the renderer, protocol, and
  eventual child-process lifecycle;
- the platform uses only Windows APIs and a small owned Rust interface;
- later launch policy can compare stable binary fingerprints instead of
  localized, mutable certificate display text.

Tradeoffs:

- this foundation does not make Launch Sample usable yet;
- Windows trust evaluation can be slower than a file read, so callers must
  invoke it off the UI thread;
- a trusted signature alone cannot establish that a mutable package was
  distributed by an approved publisher.

## Revisit conditions

Revisit when Anodrel defines signed installation records or a Windows package
identity source, when a second operating-system adapter needs an equivalent
publisher identity, or when an update system rotates publisher keys.
