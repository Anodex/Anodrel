# Decision 0118: Session window-state changes stay coalesced and pull-only

**Status:** Accepted for the next protocol slice; not implemented yet.

**Date:** 2026-08-29

## Context

Decision 0117 supplies one immediate `window.state.get` snapshot for an
authenticated session's own native window. That is enough to initialise an
owned title bar and refresh it after the application's own state request.

Anodex's existing Electron title bar also reacts when a person uses ordinary
Windows controls. Repeated snapshots would make the renderer invent a polling
loop, while an Electron-shaped subscription would add callback lifetime,
ordering, reconnection, listener detection, and continuous delivery policy to
the public platform.

## Decision

The next protocol slice is 1.31. It adds a distinct `window.state.observe`
capability and an exact `window.state.changes.read` operation. The payload is
exactly `{}`. Its result is exactly one field:

~~~json
{ "state": "minimized" | "maximized" | "restored" | null }
~~~

`null` means no state change is waiting. A non-null value is the latest state
change captured for that authenticated session's own native window since a
previous successful read. The host retains at most one value: newer changes
replace an unread value, and the first observed native state only establishes a
baseline rather than producing an event.

The operation has no target, native handle, window ID, sequence, timestamp,
event count, history, wait option, callback, subscription, delivery result,
focus, geometry, monitor, fullscreen state, or visibility field. It never
blocks waiting for a future change. A missing or expired session surface
returns `window.unavailable`.

Installed record version 1.18 will be the first version allowed to name
`window.state.observe`, as a strict superset of 1.17. It is separate from both
the write-only `window.state` and snapshot-only `window.state.read` grants.

## Consequences

- A title-bar adapter can decide its own refresh cadence and consume at most
  one current state value without a platform-owned background task.
- A busy resize cannot accumulate an unbounded event queue or expose event
  rate, ordering, timestamps, or desktop activity outside the session window.
- Windows remains responsible for observing its own window on the UI thread;
  portable code sees only the existing three-value state vocabulary.
- This is not parity with Electron's callback API. A later persistent delivery
  feature would need a different decision, transport lifecycle, and loss policy.

## Alternatives considered

**Make `window.state.get` return a change flag.** That would change a completed
snapshot contract and still make every caller poll current state.

**Add an Electron-style listener.** A listener requires lifetime, ordering,
backpressure, reconnect, and disposal policy. It is a broader capability than
the title-bar migration needs.

**Keep every transition.** A history leaks more timing information and creates
an availability queue. A title bar only needs the latest glyph state.

## Revisit conditions

Revisit before adding a wait, subscription, callback, sequence, timestamp,
history, target, focus or fullscreen observation, a secondary-view route, a
non-Windows adapter, or any event that causes application work by itself.
