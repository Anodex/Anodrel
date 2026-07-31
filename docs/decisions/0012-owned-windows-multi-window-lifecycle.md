# Decision 0012: Own per-window state before exposing a window API

**Status:** Accepted

**Date:** 2026-07-31

## Context

The direct Windows host stored its render view in one process-global slot and
ended the message loop whenever any top-level window was destroyed. This made
the initial host easy to audit, but it prevented an owned multi-window lifecycle
and would make future view routing ambiguous.

Anodrel must not solve this by adding a window framework or by treating a
future application's input as trusted native window instructions.

## Decision

The Windows host stores an immutable host-created view in an owned registry
keyed by its Win32 handle. A paint message resolves only that handle's view. A
destroy message removes only that handle; the host posts quit only when the
registry becomes empty.

The host creates a bounded list of windows before entering the User32 message
loop. It destroys already-created windows if a later creation or registration
step fails. A `--window-lab` diagnostic creates two static host-owned windows
to verify that closing one does not terminate the other.

No public window protocol, dynamic application title, cross-window messaging,
window enumeration, or native bridge is introduced.

## Consequences

Positive:

- the owned host has a real multi-window lifecycle without a framework;
- per-window painting is explicit and inspectable;
- shutdown occurs at the correct final-window boundary.

Tradeoffs:

- the host owns registry cleanup and failure rollback explicitly;
- application-driven window management remains intentionally deferred;
- direct Win32 behavior still requires manual Windows verification.

## Revisit conditions

Replace or extend this internal registry only after a documented public window
service defines identity, capability checks, lifecycle events, cancellation,
and multi-platform compatibility behavior.
