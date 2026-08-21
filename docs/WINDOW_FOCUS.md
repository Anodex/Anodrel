# Anodrel Window Focus

**Status:** Implemented in Protocol 1.20; direct Windows manual verification
remains open.

## Purpose

`window.focus.request` lets an authenticated application ask Windows to bring
the one host window for its own session to the foreground. It is a request for
attention, not a way to take over the desktop. Windows can reject it when a
person is using another application.

## Boundary

The operation does not accept or return a native handle, window ID, process ID,
coordinate, monitor, z-order, focus state, input event, callback, or retry
option. It cannot target another Anodrel session, another application, a host
diagnostic window, or a UI element. It has no read, event, subscription, or
confirmation path.

It is deliberately separate from `session.close`, `window.state.set`, and UI
Automation focus. Ending a session is not an attention request; presentation
state is not activation; and applications never control the accessibility
provider.

## Protocol

Protocol **1.20** defines exactly one operation:

| Field | Value |
| --- | --- |
| Operation | `window.focus.request` |
| Payload | `{}` |
| Grant | `window.focus` |
| Success | `{ "status": "requested" }` |
| Errors | `window.unavailable`, `window.busy` |

Unknown payload fields fail as `request.payload_invalid`; they are not a future
route for targets or input. `window.unavailable` covers no session window, a
timed-out bridge, and a Windows refusal without exposing which occurred.

An installed application record at version **1.9** may grant `window.focus`.
An earlier record that names it is invalid.

## Threading and verification

A protocol worker places at most one request in a session-owned mailbox and
waits at most five seconds. The native window's owning UI thread alone invokes
`SetForegroundWindow`; no pipe worker calls User32. A timeout clears its exact
pending slot so it cannot make a later request busy or receive a stale success.

Unit, protocol, policy, and host tests must prove the exact empty payload,
independent grant and version, unavailable/busy mapping, timeout clearing, and
that one UI-session view cannot take another view's request. The Windows
development diagnostic must be manually checked with another application in
front: Windows may flash the taskbar rather than foreground Anodrel, and either
system outcome must be treated as Windows policy rather than bypassed.
