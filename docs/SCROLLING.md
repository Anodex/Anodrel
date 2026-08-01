# Anodrel scroll-container contract

**Status:** Owned Rust model and layout contract. The external document format
remains future work.

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
inertia, scrollbars, wheel deltas, gesture input, scroll events, and persistence
are intentionally outside this contract.

## Format compatibility

`anodrel.ui.document.v1` remains exact and does not accept scroll containers.
Decision 0039 defines `anodrel.ui.document.v2` as the first exact external
scroll-container form. Its `decode_v2` and `encode_v2` codec entry points are
implemented; document-session compatibility remains separate. It will never
serialize a scroll position.
