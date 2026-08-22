# Decision 0088: Session window size is client-area only

**Status:** Accepted

**Date:** 2026-08-21

## Context

Anodrel has a native session-window title proposal, normal presentation-state
request, guarded foreground request, and reversible fullscreen request. A
desktop application also needs to make room for its own content. Calling that a
general geometry API would combine unrelated authority: resizing the client
area of an already-associated session window is not permission to learn or
choose its position, monitor, native frame, DPI, display mode, target, or
current bounds.

Applications lay out content in logical units, while Windows needs an outer
window rectangle that accounts for the current non-client frame and DPI.
`GetDpiForWindow` supplies the DPI of the known window, and
`AdjustWindowRectExForDpi` calculates an outer rectangle for a desired client
rectangle at that DPI. `SetWindowPos` can then change size while preserving
position, z-order, and activation. See Microsoft's
[GetDpiForWindow documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdpiforwindow),
[AdjustWindowRectExForDpi documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-adjustwindowrectexfordpi),
and [SetWindowPos documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos).

## Decision

Protocol 1.23 adds the separately granted `window.size` capability and one
exact `window.size.set` operation. Its payload is exactly `{ "width": integer,
"height": integer }`, where width is 320 through 3840 and height is 240
through 2160 inclusive. Both dimensions are logical 96-DPI client-area pixels.
Its only success result is `{ "status": "applied" }`.

The host resolves the native window only from the authenticated session. A
protocol worker transfers one pending size request through a five-second,
one-request UI-thread mailbox. The owning Windows UI thread gets the known
window's current DPI, derives its framed outer rectangle from the host-selected
current styles, and calls `SetWindowPos` with no move, z-order, or activation
change. It does not alter presentation state: a minimised or maximised window
may only receive an updated restored rectangle. While Anodrel fullscreen is
active for that window, the host safely declines the request rather than mixing
a new normal size into its reversible fullscreen presentation.

The operation carries no target, native handle, position, outer bounds,
monitor, DPI, display mode, state readback, event, callback, retry, constraint,
animation, focus, or visibility value. Native failures, missing session
windows, fullscreen, and expired requests map only to `window.unavailable`; a
concurrent request maps only to `window.busy`.

Installed record version 1.12 is the first allowed to name `window.size`.
Earlier records that name it are invalid.

## Consequences

- Applications gain a narrow content-sizing primitive through direct User32,
  without a browser runtime or a general geometry API.
- Logical client dimensions keep the portable contract independent of Windows
  frame metrics and per-monitor scaling, while the native host alone handles
  conversion to a framed physical rectangle.
- A success response acknowledges only an accepted request; it cannot reveal
  current presentation, geometry, display topology, or user-visible outcome.
- Fullscreen remains a separate reversible presentation mode rather than a
  hidden route to resize the desktop or change restore facts.

## Revisit conditions

Revisit before adding position, bounds or DPI readback, monitor selection,
minimum or maximum constraints, aspect ratios, resize events, animation,
window creation, cross-session targets, display-mode control, fullscreen
interaction, native handles, or a non-Windows adapter. Each changes the
authority or observation boundary and needs its own decision.
