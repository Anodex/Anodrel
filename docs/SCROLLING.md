# Anodrel scroll-container contract

**Status:** Owned model, layout, version 2 interchange, bounded Windows
host-input, and one direct-rendered first-viewport scrollbar. The Windows UI
Session Lab has an end-to-end development diagnostic for the complete path.

## Boundary

A scroll container is a portable vertical viewport with one child tree. It owns
no input device, timer, native handle, callback, protocol operation, or
application command. The host keeps its `UiScrollState` separately and supplies
the current offset to `UiDocument::layout_with_scroll_offsets`. Layout returns
one `UiScrollMetrics` record for each laid-out viewport so the host can clamp
its retained state after measuring the current viewport and content heights.

## Rules

- The viewport clips its child on every edge.
- The child is measured at the viewport width and its full intrinsic height.
- A valid offset is `0` through `contentHeight - viewportHeight`.
- Layout translates only the child vertically by the clamped offset.
- Hit testing and focus include only child portions visible through that
  viewport. The bounded accessibility tree preserves a fully clipped child with
  an empty rectangle so Windows can navigate to it without treating it as
  locally interactive.
- A nested scroll viewport is permitted only inside the existing depth and node
  limits; scroll positions are never serialized into document data.

The convenience `UiDocument::layout` method supplies no offsets, so every
scroll viewport starts at zero. Layout never mutates caller-owned positions;
it independently clamps each input before translating the child.

The first container is vertical only. Horizontal scrolling, overscroll,
inertia, gesture input, scroll events, and persistence are intentionally
outside this contract. The portable `UiScrollWheel` translates
signed input units into owned whole-line steps without retaining device state;
the Windows diagnostic and session hosts use it to accumulate partial wheel
reports locally. It is not an application input event.

The direct Windows host also overlays one scrollbar for the first visible
overflowing viewport in source order. Its track and thumb are derived from the
same current layout metrics and host-retained offset; clicking the track moves
one viewport and dragging the thumb changes only that local offset. It does not
change layout, consume a document field, move application focus, emit a
semantic event, expose a handle, or cross the protocol. Decision 0097 implements
the matching Windows UI Automation surface for that same first viewport:
vertical line, page, and percentage movement return through a bounded
host-owned route and never become application state. Decision 0098 uses that
same route for `ScrollIntoView` on eligible descendants of that viewport,
without a position or alignment input. Nested-scrollbar arbitration remains
separate. See `docs/UI_AUTOMATION_SCROLL.md` and
`docs/UI_AUTOMATION_SCROLL_ITEMS.md`.

The `--sample-ui-scroll-client` Windows development command sends one exact v2
tree whose only enabled action begins below the initial viewport. An operator
uses local mouse-wheel or Page Down input to reveal it, then activates it. The
authenticated client receives only that revision-and-action candidate through
the existing pull operation and requests close for its own session. This proves
the path without serializing a position, exposing a scroll event, or giving the
application a native input handle.

## Format compatibility

`anodrel.ui.document.v1` remains exact and does not accept scroll containers.
Decision 0039 defines `anodrel.ui.document.v2` as the first exact external
scroll-container form. Its codec, explicit session replacement path, and
authenticated `ui.document.replace.v2` operation are implemented. It will
never serialize a scroll position.

Decision 0102 extends the same exact v2 form to bounded secondary session views
through Protocol 1.27 `window.open.v2` and
`ui.document.replace.window.v2`. A secondary's retained position, pointer,
keyboard, and automation scrolling remain local to that one host-owned view;
there is no cross-view or application-observable scroll route.
