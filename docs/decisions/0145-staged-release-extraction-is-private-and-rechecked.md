# Decision 0145: Stage release files privately and recheck them before publication

**Status:** Accepted

**Date:** 2026-08-31

## Context

The owned bundle decoder authenticates every entry while it borrows bytes from
the signed installer image. Installation still needs a filesystem boundary:
Windows must receive regular files beneath a new private directory before an
installed-record value can select that directory.

Writing directly into a selected version directory, accepting a target path, or
publishing a registry record while extraction is incomplete would make a crash
or malformed release visible to the product host. Bundle path rules are
portable, but Windows has additional device-name, case-collision, and path
normalization hazards that extraction must reject instead of leaving to the
filesystem.

## Decision

Build a first-party staging module inside `anodrel-windows-installer`. It
accepts only a caller-selected absolute application staging parent and an
already checked release manifest plus bundle. It creates one previously absent
private staging directory, derives child paths from the bundle's canonical
forward-slash paths, writes each regular file with create-new semantics, syncs
it, rehashes the resulting bytes, and rejects unsafe Windows path components.

The module renders the existing version-1.19 installed record for that new
directory and requires `anodrel-application` to validate the package identity,
content, executable containment, and executable digest before it returns a
staged result. A failure removes only the staging directory it created; it
never alters an existing version directory or registry record.

The selected parent is installer-owned machine data. Version 1 assumes that
the elevated installer creates and retains it below Program Files, where
non-administrative users cannot replace its children. Administrators remain in
the machine trust boundary. Registry publication, target-version promotion,
stale-directory recovery, and executable Authenticode verification are separate
subsequent transactions.

## Consequences

Positive:

- Every installed file is checked once from the signed bundle and again after
  it reaches the staging filesystem.
- A product host cannot select a partially extracted release because no policy
  record is published by this step.
- Filesystem behaviour stays testable with a temporary installer-owned parent;
  no production location or elevation is required for the unit boundary.

Tradeoffs:

- Version 1 rejects otherwise portable names that Windows normalizes or treats
  as devices.
- Rechecking files costs one additional sequential installer-time read.
- A crash can leave a private stale staging directory; later recovery must
  remove only directories with this exact ownership format.

## Revisit conditions

Revisit for a brokered installer, a user-scoped package root, package formats
with signed per-file metadata, or a measured need for streaming extraction.
