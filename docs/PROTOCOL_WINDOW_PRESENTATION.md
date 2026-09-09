# Anodrel session-window presentation operation reference

This companion to [the Protocol v1 operation reference](PROTOCOL_OPERATIONS.md)
defines the two presentation operations that address only the authenticated
session's primary native window. Neither operation can select another window or
expose native handles, coordinates, monitors, callbacks, or desktop topology.

## Session-window foreground request

Protocol 1.20 implements `window.focus.request` behind its separate
`window.focus` grant. It accepts exactly `{}` and returns exactly
`{ "status": "requested" }` when Windows accepts the owning host UI thread's
foreground request for that authenticated session's own window. The operation
cannot name a target, window handle, process, coordinate, monitor, input,
retry policy, callback, or accessibility element.

Windows may decline a foreground request under its own user-protection policy.
`window.unavailable` deliberately covers a missing session window, an expired
UI bridge, and such a refusal without revealing which occurred. The response
does not state whether the window became foreground, received keyboard focus,
moved in z-order, or was noticed by a person. A concurrent request returns
`window.busy`. See `docs/WINDOW_FOCUS.md` and Decision 0085.

## Session-window fullscreen request

Protocol 1.21 implements `window.fullscreen.set` behind its separate
`window.fullscreen` grant. It accepts exactly `{ "mode": "fullscreen" | "windowed" }`
and returns exactly `{ "status": "applied" }` when the owning host UI thread
accepts the requested reversible presentation action for that authenticated
session's own window.

The operation cannot name a target, window handle, process, monitor,
coordinate, size, style, display mode, z-order, visibility, keyboard shortcut,
callback, or accessibility element. On Windows, `fullscreen` means borderless
windowed fullscreen on the monitor Windows associates with that known window;
it is not exclusive display control. The host retains restoration facts
privately and `windowed` restores them. Duplicate requests for the current host
mode are accepted without revealing that mode.

`window.unavailable` covers a missing session window, expired bridge, and a
safe native-transition failure without revealing which occurred. A concurrent
request returns `window.busy`. The response never states resulting bounds,
monitor, style, visibility, or fullscreen state. See
`docs/WINDOW_FULLSCREEN.md` and Decision 0086.
