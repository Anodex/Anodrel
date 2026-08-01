# Anodrel scroll-container contract

**Status:** Design contract for the next UI document format.

## Boundary

A scroll container is a portable vertical viewport with one child tree. It owns
no input device, timer, native handle, callback, protocol operation, or
application command. The host keeps its `UiScrollState` separately and supplies
the current offset to layout.

## Rules

- The viewport clips its child on every edge.
- The child is measured at the viewport width and its full intrinsic height.
- A valid offset is `0` through `contentHeight - viewportHeight`.
- Layout translates only the child vertically by the clamped offset.
- Hit testing, focus, and accessibility include only the child portions visible
  through that viewport.
- A nested scroll viewport is permitted only inside the existing depth and node
  limits; scroll positions are never serialized into document data.

The first container is vertical only. Horizontal scrolling, overscroll,
inertia, scrollbars, wheel deltas, gesture input, scroll events, and persistence
are intentionally outside this contract.

## Format compatibility

`anodrel.ui.document.v1` remains exact and does not accept scroll containers.
The first external scroll-container form will use a new exact format identifier
and tests that prove v1 remains unchanged.
