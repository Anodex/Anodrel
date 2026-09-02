# Decision 0167: Verified update images stream to fresh private files

**Status:** Accepted

**Date:** 2026-09-01

## Context

An attached CMS catalogue can bind one future installer image, but retaining a
576 MiB image in memory would be wasteful. Accepting a URL or file name from an
application would also turn an updater into a general download-and-execute
surface. A verified catalogue alone is insufficient: its application identity,
publisher, and version must remain continuous with fixed installed Windows
policy before an image is fetched.

## Decision

Create one direct Windows update-download adapter. It accepts only the opaque
result of attached-CMS catalogue verification, reloads fixed installed policy
and Authenticode facts, and requires exact identity, publisher continuity, and
a strictly newer canonical release version before it prepares a transfer.

It makes one shared direct WinHTTP `GET` with required status 200 and streams
the declared bounded image into one fresh regular file below an updater-owned
canonical cache directory. It calculates SHA-256 while writing, synchronizes
the file, and compares the final descriptor without exposing the expected
digest. All failures and ordinary drop remove only that fresh file.

## Consequences

- The updater has no general download authority and cannot use parsed unsigned
  catalogue data.
- Update images remain bounded by the 64 KiB transfer buffer rather than their
  total declared size.
- Download success is deliberately not installation acceptance; existing
  Authenticode and installer gates still run immediately before update.
- Catalogue discovery, cache-root selection, user consent, elevation handoff,
  process launch, scheduling, recovery, and certificate rotation remain
  explicit later boundaries.

## Revisit conditions

Revisit for resumable downloads, multiple candidates, delta images, another
cache root, another operating system, concurrent or background transfers,
network endpoint discovery, user-visible progress, automatic updates, or a
different installer format.
