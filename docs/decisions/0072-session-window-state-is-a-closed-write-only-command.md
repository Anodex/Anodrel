# Decision 0072: Session window state is a closed, write-only command

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0012 gave the Windows host a safe private lifecycle for several native
windows. Decision 0066 exposed only title proposals, because a title is the
smallest useful public operation and needed a separate answer to impersonation.

Applications still need ordinary presentation control. Asking to minimise a
document while someone takes a call, maximise a workspace, or restore it is not
the same authority as creating a window, reading its placement, forcing focus,
or moving one under a pointer. Treating them all as "window management" would
either delay the useful small actions indefinitely or ship a broad ambient
native bridge.

## Decision

Protocol 1.16 introduces the separately granted `window.state` capability and
one exact operation, `window.state.set`. Its payload selects exactly one of
`minimized`, `maximized`, or `restored`.

The request carries no window target, identifier, native handle, geometry,
focus option, or read flag. The host resolves the native window from the
authenticated session and routes the closed value through a one-request,
five-second UI-thread mailbox. The host reports only acceptance, busy, or safe
unavailability; it does not report the present or resulting window state.

Installed record version 1.6 is the first version that can name
`window.state`. Older records naming it are invalid.

## Consequences

Applications gain the standard minimise, maximise, and restore controls for
their own host window without any cross-window authority or a path to inspect
native state. The direct Windows adapter remains the only layer that calls the
operating-system window API; the core sees a portable closed enum and service
interface.

There is intentionally no fullscreen, always-on-top, hide, show, geometry,
state readback, event, subscription, focus, close, or multi-window creation
operation. Each changes a different security or compatibility boundary and
must be reviewed separately.

## Alternatives considered

**Expose a general window object or native handle.** Convenient for an
application, but it makes every later action targetable and leaks host topology
before its permissions and lifecycle rules exist. Refused.

**Use `session.close` for minimise.** Closing a session ends resources and may
end its verified child; minimising must be reversible and cannot mean the same
thing. Refused.

**Return the resulting native state.** A result may be affected by shell policy
or another actor. Returning it would create a host-state observation channel
and is unnecessary to request the action. Refused.

**Add bounds and fullscreen at the same time.** They require monitor, size,
placement, and escape-behaviour policies that are unrelated to the three
standard presentation states. Refused.

## Revisit conditions

Revisit this decision only when a concrete application need requires a new
window property. It must introduce an explicit grant, versioned protocol
contract, thread/lifecycle rule, compatibility tests, and a threat-model entry;
it must not extend this payload with an ambient target or arbitrary native
command.
