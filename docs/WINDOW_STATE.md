# Anodrel Window State

**Status:** Implemented on Windows. One authenticated session may request a
presentation state for the native window it already owns. Manual desktop
verification remains open.

## Purpose

An application needs a small amount of ordinary desktop-window behaviour before
it needs window management. A document can be set aside, expanded to use the
screen, and returned to a normal window without learning a handle, its bounds,
or whether any other window exists.

`window.state.set` deliberately provides only those three host-applied actions:

| Requested state | Host action |
| --- | --- |
| `minimized` | Minimise the session's own native window. |
| `maximized` | Maximise the session's own native window. |
| `restored` | Restore the session's own native window. |

The request has no target field. The host finds the window from the
authenticated session, just as it does for `window.title.set` and
`session.close`. An application therefore cannot aim the action at another
application, another session, or a host diagnostic window.

## Boundary

This is a write-only presentation request, not a window-inspection or
management API. It does **not** let an application:

- create, close, enumerate, identify, read, move, resize, focus, hide, show,
  or inject into a window;
- learn the current or resulting state, position, size, monitor, z-order, or
  native handle;
- receive a state-change event, activation event, or subscription;
- set a window to fullscreen, always-on-top, modal, frameless, or any other
  state not named by this contract.

`session.close` remains the separately granted request to end the requesting
session. A minimise request does not close a session, and a restore request
does not create a window that has already closed.

## Protocol

Protocol **1.16** adds exactly one operation:

| Field | Value |
| --- | --- |
| Operation | `window.state.set` |
| Payload | `{ "state": "minimized" \| "maximized" \| "restored" }` |
| Grant | `window.state` |
| Success | `{ "status": "applied" }` |
| Errors | `window.unavailable`, `window.busy` |

The payload must have exactly one `state` field. A target, handle, identifier,
geometry, focus option, or unknown state is a `request.payload_invalid` error;
unknown fields are not silently accepted as a future escape hatch.

Success means the host's owning UI thread accepted and issued the documented
native action. It does not say that the action became visible, that the window
has a particular geometry, or that a person saw it. Returning any of those
facts would create a readback channel for host state that this capability does
not need.

Installed application record version **1.6** adds `window.state` as a strict
superset of version 1.5. A record at any earlier version naming this grant is
invalid, so stale provisioning cannot silently widen an application's authority.

## Threading and failure behaviour

The protocol worker never calls User32. It hands one closed state value to a
per-session mailbox, and the native UI thread performs the action on the
window it owns. At most one request may be pending. A second request while the
first is in flight fails as `window.busy`; a request with no associated native
window, an expired UI-thread bridge, or a host-side failure is
`window.unavailable`.

The bridge has the same five-second bound as the title and notification
bridges. When the UI thread does not answer in time, the pending slot is
cleared before the safe unavailable result is returned. One stuck request
cannot leave a session permanently busy.

## Verification

Portable, core, policy, and protocol-contract tests prove the closed state
grammar, exact payload shape, independent version and grant checks,
unavailable/busy mapping, record-version widening guard, and one-request bridge
timeout. Windows-host tests prove only the associated session view can take or
complete a command and that every portable value maps to its documented User32
command.

The remaining manual Windows verification runs an authenticated development
session, observes minimise, maximise, and restore, then closes it normally; no
other host window may change. `docs/DEVELOPMENT_DIAGNOSTICS.md` gives the exact command.

## Compatibility

This capability is complete as specified. Reading state, setting bounds,
foregrounding, creating another window, targeting a window, and lifecycle
events are each separate capabilities with their own protocol version, grant,
decision, and threat-model entry. The separate `window.focus.request` contract
now covers only an authenticated session asking Windows to foreground its own
window; it does not expand this state command. The private multi-window host foundation in
`docs/WINDOW_LIFECYCLE.md` remains private; this contract does not expose it.
