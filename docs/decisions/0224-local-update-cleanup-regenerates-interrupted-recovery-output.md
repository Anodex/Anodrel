# Decision 0224: Local update cleanup regenerates interrupted recovery output

**Status:** Accepted

**Date:** 2026-09-13

## Context

The fixed local update fixture creates a fresh signed retirement image only to
retire its own exited helper cache and retained rollback package after signed
uninstall. An interrupted `-Remove` run can leave that private fixture output
behind. Reusing it would make a later cleanup trust bytes from an earlier,
incomplete operator operation; refusing it forever would strand temporary
development trust despite the installed policy and packages already being gone.

## Decision

When policy is absent and the fixed application root still requires cleanup,
the elevated fixture script may remove only its exact
`%LOCALAPPDATA%\Anodrel\LocalUpdateFixture\retirement` directory after proving
that the path and every nested entry contain no reparse point. It then builds a
new signed retirement image from the fixture's existing fixed publisher
identity and invokes only that image's fixed `cleanup-cache` command.

The script never reuses a prior retirement image, accepts a cleanup path,
deletes an installed package directly, or changes selected policy. The signed
native retirement command continues to validate its own image, fixed
application identity, publisher, cache contents, and retained prior record
before it can reclaim protected Program Files content.

## Consequences

- Interrupted fixture cleanup can resume without retaining stale local output.
- A malformed or reparse-point-containing retirement directory still fails
  closed and retains development trust for investigation.
- Production cleanup and the generic signed-helper contract are unchanged.

## Revisit conditions

Revisit for a production recovery-image store, a different fixture ownership
boundary, a persistent installer service, or a non-Windows update route.
