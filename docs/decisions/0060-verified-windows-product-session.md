# Decision 0060: Join verified Windows product lifetimes in one host-owned session

**Status:** Accepted

**Date:** 2026-08-08

## Context

The Windows registered-session adapter creates the authenticated pipe and the
native UI resources from a machine-validated installed record. The locked
launch service separately starts only the record's verified executable and
returns a tracked child. Neither component owns the complete product lifetime.

Leaving a host to connect those objects ad hoc risks orphaning a child when the
pipe fails, leaving a pipe worker blocked when the child exits before
authentication, or attaching a native window to a mismatched session.

## Decision

Add `anodrel-windows-product-session`, a host-only Windows lifecycle adapter.
It creates one registered interactive session, converts its private invitation
for bootstrap delivery, runs the locked verified launch, and starts exactly one
pipe worker and one child-exit watcher.

The pipe worker requests native-window closure and terminates the tracked child
when the one-client connection ends. The exit watcher requests pipe shutdown
and native-window closure when the child exits. Explicit shutdown performs the
same three actions. The adapter returns the one grouped UI resource set only
through its running-session object; applications cannot create, control, or
inspect this lifecycle.

## Consequences

Positive:

- the verified child, authenticated pipe, and host window share one shutdown
  owner;
- both early child failure and broken IPC close the product session safely;
- the coordinator introduces no protocol operation, command argument,
  application-selected title, or runtime dependency.

Tradeoffs:

- full integration verification requires a signed executable and
  machine-provisioned record, which this repository does not yet ship;
- product window policy remains host-selected and single-window for now.

## Revisit conditions

Revisit when product sessions gain multi-window ownership, restart policy,
graceful application shutdown negotiation, background processes, or a
cross-platform lifecycle contract.
