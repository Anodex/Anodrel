# Decision 0147: Promote a verified stage with one same-volume rename

**Status:** Accepted

**Date:** 2026-08-31

## Context

A release that has passed installer, bundle, package, executable digest, and
publisher checks is still private staging data. It needs a durable version
directory before a later registry transaction can select it, but installation
must never overwrite an existing version or copy partially validated files into
that destination.

The staging and version directories are siblings below one installer-owned
application root. Windows `MoveFileExW` moves directories only on the same
drive; without `MOVEFILE_COPY_ALLOWED` it has no cross-volume copy-and-delete
fallback. Omitting `MOVEFILE_REPLACE_EXISTING` preserves any existing version
directory.

## Decision

Build an owned promotion boundary. It accepts only a private stage produced by
the prior signed-release preparation gate. It derives the final directory name
from the signed three-part package version, requires that sibling destination
to be absent, then calls `MoveFileExW` with no copy or replacement flags.

On failure the stage remains private and is cleaned up through its existing
ownership guard. On success a `PromotedRelease` retains the complete final
directory and its already validated record, but this boundary writes no
registry value. A crash before later registry publication therefore leaves
either the previous record or an unselected complete version directory; it
never selects an incomplete stage.

## Consequences

Positive:

- Promotion neither overwrites a version nor falls back to a cross-volume copy.
- The final path comes from bounded signed version numbers, never application
  or command-line input.
- Registry authority remains a distinct final publish transaction.

Tradeoffs:

- A stale complete but unselected version can remain after a later publish
  failure and requires a dedicated recovery policy.
- The application root must be installer-owned and reside on one local volume.

## Revisit conditions

Revisit for content-addressed release directories, package identities that
replace filesystem versions, or a brokered installation service.
