# Decision 0018: Bind launch policy through an external installed application record

**Status:** Accepted

**Date:** 2026-07-31

## Context

The Windows signature adapter can report an Authenticode leaf certificate
fingerprint for a successfully verified executable, while the application
package loader can validate a text package's identity and content. Neither
establishes whether that executable is approved for that application. A
manifest placed in the same mutable package directory cannot provide approval.

The direct bootstrap launcher intentionally accepts a caller-selected
executable. Reusing it as a product-launch API before an external policy binds
the executable, package, and publisher would create arbitrary local process
authority.

## Decision

Define an exact installed application record, held outside the package in a
trusted host policy directory. The record binds a validated application ID to
an absolute package root, one contained `.exe` path and SHA-256 digest, and one
approved Authenticode leaf certificate fingerprint.

The portable `anodrel-application` foundation parses the record, verifies its
location relative to a host-selected policy root, loads the existing package,
requires the application IDs to match, canonicalizes and contains the
executable, and checks its bounded digest. It never selects the policy root,
uses a record path supplied by an application, calls Authenticode, starts a
process, exposes policy values, or grants a capability.

A later Windows policy-store adapter establishes the trusted policy root and
its modification rules. A later launch service revalidates the executable,
matches the Windows Authenticode result to the record, creates and tracks the
child without a shell, and delivers bootstrap material only after every check.

## Consequences

Positive:

- publisher authorization is no longer inferred from mutable package data;
- package, executable, signer, process, and bootstrap boundaries remain small
  and independently testable;
- the record schema is portable even though its first trusted store is Windows;
- no third-party runtime or installer framework is introduced.

Tradeoffs:

- this does not make the Launch Sample action available yet;
- a Windows policy-store adapter needs a documented installation and
  access-control model;
- the launch service must account for revalidation and process-creation races.

## Revisit conditions

Revisit when Anodrel adopts an operating-system package identity, a signed
installation-record format, publisher-key rotation, or another operating
system needs an equivalent trusted policy source.
