# Decision 0093: Closing the primary session view ends its whole group

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0092 establishes a session-owned group with one primary view and up
to three secondary views. It deliberately left primary-window closure open
until the direct Windows host could own the actual native mappings.

Letting a person close the primary native window while its session continued in
secondary windows would leave every current targetless service (title, state,
focus, fullscreen, size, menu, dialog, notification, field read, and retained
file selection) bound to a logical `main` view that no longer exists on the
desktop. It would also make a verified product child outlive the primary
surface that is responsible for its session-wide lifecycle. Reassigning those
services to a secondary would be an implicit new authority and an observable
lifecycle policy, neither of which has a contract.

## Decision

The primary view is the session anchor. If Windows destroys its native window,
the host requests group-wide shutdown. Every remaining native window in that
one session observes the host-owned close state and is destroyed on the owning
UI thread. A `session.close` request uses the same path.

Destroying a secondary removes only its exact logical view after its native
window is gone. It does not affect the primary or sibling views. Removal is
idempotent: late destruction, panic cleanup, and a group-wide close can all
reach the same mapping without reviving a closed identity.

The native group retains a verified product session, when present, until the
last registered group view has left the host registry. Thus the product child,
pipe worker, and exit watcher cannot be released merely because the first of
several windows closes, but they are still shut down when the group ends.

An in-flight worker-to-UI open request is cancelled as soon as group shutdown
starts. Its reserved logical identity is rolled back and the waiting worker is
answered with the existing unavailable category rather than waiting for its
five-second deadline.

## Consequences

Positive:

- current primary-only services never become detached from a real primary
  surface;
- product lifetime remains group-owned without stranding a child behind a
  closed first window;
- secondary close and group shutdown have clear, independently testable
  effects; and
- no native handle, close reason, window enumeration, or lifecycle event is
  added to the application protocol.

Tradeoffs:

- an application cannot keep secondary windows alive after a person closes its
  primary window;
- making a secondary an independently promotable primary requires a new
  protocol and lifecycle decision.

## Revisit conditions

Revisit before adding primary promotion, background execution, restoration,
restart, a close-veto flow, an application-visible lifecycle event, or any
secondary-window version of a current primary-only native service.
