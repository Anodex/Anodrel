# Anodrel Session Window Fullscreen

**Status:** Implemented in Protocol 1.21; direct Windows manual verification
remains open.

## Purpose

`window.fullscreen.set` lets an authenticated application choose between the
normal windowed presentation of the one host window for its own session and a
borderless fullscreen presentation of that same window.

It is deliberately *not* an exclusive display-mode API. The host does not
change a display's resolution, refresh rate, colour mode, ownership, or power
state. It does not select a monitor supplied by an application. On Windows,
the host chooses the monitor that already contains the session window and
covers that monitor's bounds with the existing window.

## Boundary

The operation does not accept or return a native handle, window ID, process ID,
monitor, coordinate, size, style, display mode, z-order, visibility, current
fullscreen state, keyboard shortcut, callback, or retry option. It cannot
target another Anodrel session, another application, a host diagnostic window,
or a UI element. It has no read, event, subscription, or confirmation path.

The application can select only one of two closed modes:

| Requested mode | Host action |
| --- | --- |
| `"fullscreen"` | Apply borderless fullscreen on the host-selected monitor and privately retain the pre-fullscreen presentation facts. |
| `"windowed"` | Restore the facts retained by the host for that same window, if it is fullscreen. |

Both requests are idempotent. A request for the mode already applied succeeds
without revealing that it was already applied. A success result means only that
the host applied the requested presentation action; it does not reveal the
resulting bounds, monitor, style, visibility, or any other desktop state.

`window.fullscreen.set` is separate from `window.state.set`. Minimise,
maximise, and restore are normal Windows presentation commands; fullscreen
requires reversible style and placement handling. It is also separate from
`window.focus.request`: changing presentation does not ask to steal foreground
attention or bypass Windows foreground policy.

## Protocol

Protocol **1.21** defines exactly one operation:

| Field | Value |
| --- | --- |
| Operation | `window.fullscreen.set` |
| Payload | `{ "mode": "fullscreen" \| "windowed" }` |
| Grant | `window.fullscreen` |
| Success | `{ "status": "applied" }` |
| Errors | `window.unavailable`, `window.busy` |

The payload must be an object with exactly the one `mode` field. Unknown,
missing, duplicate, or wrongly typed fields fail as `request.payload_invalid`;
they are not a future route for monitor selection, display control, geometry,
or input.

An installed application record at version **1.10** may grant
`window.fullscreen`. An earlier record that names this grant is invalid.

## Native behavior and verification

A protocol worker places at most one request in a session-owned mailbox and
waits at most five seconds. The native window's owning UI thread alone changes
that window's presentation. On first entry to fullscreen, the Windows adapter
records the window's normal placement and style privately, removes only the
normal framed-window presentation, and covers the monitor Windows identifies
for that window. On exit, it restores the retained style and placement through
the matching Windows placement API. Neither the retained data nor an operating
system failure crosses the protocol boundary.

If the adapter cannot capture, apply, or restore the presentation safely, it
reports only `window.unavailable`. It makes a best-effort restoration before
reporting failure. A concurrent request reports `window.busy`. A timeout clears
its exact pending slot, so it cannot make a later request busy or receive a
stale success.

Unit, protocol, policy, and host tests must prove the exact closed payload,
independent grant and record version, unavailable/busy mapping, timeout
clearing, idempotent restore model, and that one UI-session view cannot take
another view's request. Direct Windows verification must demonstrate entry,
return to the original framed placement, and use of the monitor that contained
the session window on a multi-monitor desktop.

## Deferred work

Exclusive fullscreen, monitor selection, display-mode changes, a fullscreen
state read or event, keyboard shortcuts, topmost control, geometry, window
creation, and non-Windows adapters each require their own protocol version,
grant, decision, threat-model entry, and verification.
