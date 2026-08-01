# Decision 0036: Session close uses a host-owned coalescing signal

**Status:** Accepted

**Date:** 2026-08-01

## Context

An application can now receive a semantic action from its authenticated native
view, but it has no way to finish that session without relying on a developer
to close a host window. Giving the application a window handle or a general
close-by-ID operation would break the host's ownership boundary and make one
session capable of targeting another view.

## Decision

Protocol 1.3 adds capability-gated `session.close`. It accepts exactly an empty
payload and returns only `{ "status": "accepted" }`. The core sets one shared,
host-created `SessionCloseSignal`; it has no queue, payload, target, native
handle, or operating-system behavior. The transport returns the response in
the normal request/response flow. A host that supplied the signal may consume
it on its UI or lifecycle thread and close resources belonging to that one
session.

## Consequences

- applications can end their own granted session without a native bridge;
- a close request cannot select, enumerate, focus, or manipulate windows;
- repeated requests are bounded and idempotent; and
- product window lifecycle, close confirmation, multi-window targeting, and
  process termination remain separate future contracts.

## Revisit conditions

Revisit before adding a reason string, window identity, lifecycle event,
process exit behavior, close confirmation, multiple-session coordination, or
any public window-management operation.
