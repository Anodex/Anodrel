# Decision 0086: Session window fullscreen is borderless and restorable

**Status:** Accepted

**Date:** 2026-08-21

## Context

Anodrel's authenticated session window can already receive a bounded title
proposal, a closed normal presentation-state command, and a guarded foreground
request. Applications also need a way to use the full usable display for their
own immersive workflow. Treating that need as a general window-management or
display API would incorrectly bundle very different authority: choosing another
monitor, changing a display mode, naming a native window, reading desktop
topology, and changing a session window's reversible presentation are not the
same power.

The first Windows adapter needs to restore a framed window precisely after
fullscreen. Microsoft documents `WINDOWPLACEMENT` as the matching state for
`GetWindowPlacement` and `SetWindowPlacement`, including its required `length`
field and its workspace-coordinate semantics. The adapter must also obtain the
monitor from the known native window rather than from primary-screen metrics so
the window covers the monitor it already occupies. See Microsoft's
[WINDOWPLACEMENT documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-windowplacement)
and [multiple-monitor guidance](https://learn.microsoft.com/en-us/windows/win32/gdi/positioning-objects-on-multiple-display-monitors).

## Decision

Protocol 1.21 adds the separately granted `window.fullscreen` capability and
one exact `window.fullscreen.set` operation. Its payload is exactly
`{ "mode": "fullscreen" | "windowed" }`, and its success result is exactly
`{ "status": "applied" }`.

The host resolves the native window only from the authenticated session. A
protocol worker transfers one pending mode through a five-second, one-request
UI-thread mailbox. The owning UI thread alone applies a **borderless windowed
fullscreen** transition on the monitor associated with that known window. It
does not enter exclusive fullscreen or alter a display mode. On first entry,
the adapter captures the window's style and placement privately. On exit, it
restores those exact facts through the matching Windows placement API. Duplicate
requests for the currently applied mode are accepted without exposing current
state.

The operation carries no target, native handle, monitor, coordinate, display
mode, z-order, state readback, event, input, callback, retry, or topmost flag.
Native failures, missing session windows, and expired requests map only to
`window.unavailable`; a concurrent request maps to `window.busy`.

Installed record version 1.10 is the first record version allowed to name the
new `window.fullscreen` grant. Earlier record versions that name it remain
invalid.

## Consequences

- Applications can request reversible fullscreen for their own session window
  through direct User32 and monitor APIs, without a browser or runtime
  dependency.
- The host, not application code, owns all native style and placement facts,
  which prevents a request from becoming geometry or desktop-topology readback.
- A later request for exclusive display control, monitor selection, or
  fullscreen state observation is not a parameter addition; it changes the
  authority boundary and must be designed separately.

## Revisit conditions

Revisit before adding exclusive fullscreen, a display-mode change, monitor
selection, geometry, z-order or topmost control, a keyboard shortcut, focus or
visibility behavior, a state read/event/subscription, window creation, a
native or application-supplied target, or a non-Windows adapter. Each changes
the authority or observation boundary and needs its own decision.
