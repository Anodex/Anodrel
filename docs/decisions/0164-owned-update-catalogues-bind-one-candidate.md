# Decision 0164: Owned update catalogues bind one candidate before retrieval

**Status:** Accepted

**Date:** 2026-08-31

## Context

The owned installer can safely verify and promote an already selected newer
signed release, but a direct updater needs small bounded data that names an
exact candidate image before it downloads hundreds of megabytes. Trusting a
mutable HTML page, arbitrary URL, redirect, or external feed framework would
weaken the release boundary and make rollback or publisher confusion easier.

## Decision

Define strict `anodrel.update-catalogue.v1` data with one application identity,
one publisher fingerprint, one release version, and one HTTPS installer
location plus exact byte length and SHA-256. Build a portable first-party parser
that bounds and validates these fields, compares them to host-held installed
identity/publisher/version facts, and checks downloaded image bytes without
exposing the expected digest.

The parser performs no trust check or I/O. Before a later Windows updater may
use a catalogue, a dedicated direct Windows message-signature boundary must
authenticate it to the installed publisher. The later HTTPS boundary and the
existing signed-installer update transaction remain separate.

## Consequences

- Update retrieval can have one small, strict, independently tested authority
  format without importing an updater framework or archive format.
- The image's digest and length provide an early bounded download check; they
  do not replace the existing Authenticode and embedded-release checks.
- A signed catalogue could still be stale or unavailable, so automatic update
  timing, rollback policy, key rotation, and delivery source remain open work.

## Revisit conditions

Revisit for multiple release channels, a release history rather than one
candidate, publisher rotation, delta delivery, enterprise deployment metadata,
or an approved platform package route.
