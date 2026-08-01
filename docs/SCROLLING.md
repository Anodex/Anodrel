# Anodrel scroll-container contract

**Status:** Owned model, layout, version 2 interchange, and bounded Windows
host-input contract.

## Boundary

A scroll container is a portable vertical viewport with one child tree. It owns
no input device, timer, native handle, callback, protocol operation, or
application command. The host keeps its `UiScrollState` separately and supplies
the current offset to `UiDocument::layout_with_scroll_offsets`. Layout returns
one `UiScrollMetrics` record for each visible viewport so the host can clamp
its retained state after measuring the current viewport and content heights.

## Rules

- The viewport clips its child on every edge.
- The child is measured at the viewport width and its full intrinsic height.
- A valid offset is `0` through `contentHeight - viewportHeight`.
- Layout translates only the child vertically by the clamped offset.
- Hit testing, focus, and accessibility include only the child portions visible
  through that viewport.
- A nested scroll viewport is permitted only inside the existing depth and node
  limits; scroll positions are never serialized into document data.

The convenience `UiDocument::layout` method supplies no offsets, so every
scroll viewport starts at zero. Layout never mutates caller-owned positions;
it independently clamps each input before translating the child.

The first container is vertical only. Horizontal scrolling, overscroll,
inertia, scrollbars, gesture input, scroll events, and persistence are
intentionally outside this contract. The portable `UiScrollWheel` translates
signed input units into owned whole-line steps without retaining device state;
the Windows diagnostic and session hosts use it to accumulate partial wheel
reports locally. It is not an application input event.

## Format compatibility

`anodrel.ui.document.v1` remains exact and does not accept scroll containers.
Decision 0039 defines `anodrel.ui.document.v2` as the first exact external
scroll-container form. Its codec, explicit session replacement path, and
authenticated `ui.document.replace.v2` operation are implemented. It will
never serialize a scroll position.
