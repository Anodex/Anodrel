# Anodrel Window State Changes

**Status:** Specified for Protocol 1.31; not implemented yet.

## Purpose

`window.state.changes.read` is the planned companion to the immediate
`window.state.get` snapshot. It lets an owned title bar notice the most recent
ordinary Windows presentation change without receiving a native handle or a
continuous event stream.

## Planned contract

| Field | Value |
| --- | --- |
| Protocol | 1.31 or later |
| Operation | `window.state.changes.read` |
| Payload | `{}` exactly |
| Grant | `window.state.observe` |
| Success | `{ "state": "minimized" \| "maximized" \| "restored" \| null }` |
| Error | `window.unavailable` |

The host captures state changes only for the authenticated session's own
native window. It retains one latest unread state. A later state replaces an
earlier unread one, so the operation has no queue length, dropped count,
history, sequence, timestamp, or ordering guarantee. `null` means that no
state change is waiting; it is not an assertion that the window is restored.

The first native state observed after a view appears establishes the host's
baseline and does not produce a change. Use `window.state.get` when an initial
state is required.

## Boundaries

This proposed API accepts no selector, target, window ID, native handle,
position, bounds, monitor, DPI, fullscreen value, focus, visibility, z-order,
wait duration, callback, subscription, event count, history, or delivery
confirmation. It never waits for a later change and never starts a background
receiver. An application chooses whether and when to make another ordinary
request.

`window.state.observe` will be independent from `window.state` and
`window.state.read`. A policy can therefore allow a title-bar refresh without
allowing a presentation request, or allow a one-time snapshot without granting
change observation. Record version 1.18 will be the first version that can
name the new grant.

## Anodex migration use

Anodex's Electron title bar currently has an `onMaximizedChanged` callback.
The planned Anodrel adapter will not imitate that callback with a hidden
background loop. It can expose one explicit refresh method that maps the
latest coalesced state to the same maximise/restore glyph. Any future
persistent listener needs its own platform contract.

See Decision 0118 and `docs/WINDOW_STATE_OBSERVATION.md`.
