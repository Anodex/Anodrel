# Decision 0038: Scroll containers use host-retained state and layout metrics

**Status:** Accepted

**Date:** 2026-08-01

## Context

The owned `UiScrollState` can safely clamp a vertical position, but a usable
native UI tree also needs a viewport node, a deterministic translation rule,
and a way for hosts to react to changing content or client sizes.

## Decision

`anodrel-ui` adds an in-memory `Scroll` node with one child and a stable
element ID. `UiDocument::layout_with_scroll_offsets` accepts a host-retained
map of `UiScrollState` values keyed by that ID. It does not mutate the map:
each layout pass independently clamps the supplied offset, clips the child on
all edges of the viewport, and returns `UiScrollMetrics` with the viewport and
content heights required for the host's next retained-state update.

The existing `layout` method remains the zero-offset convenience form. The
scroll node is an owned Rust model feature only; `anodrel.ui.document.v1`
continues to reject it for both decoding and encoding until a new exact format
defines its external representation.

## Consequences

- layout, input routing, and native rendering remain separate, testable
  boundaries;
- a stale host position cannot expose content outside the current measured
  range during layout;
- hit testing, focus, and accessibility naturally use only the visible clipped
  layout items; and
- scrollbars, wheel input, gestures, persistence, horizontal scrolling, and
  application-facing scroll events remain follow-up work.

## Revisit conditions

Revisit before adding a new external document format, mutable layout state,
scroll input routing, nested-scroll arbitration, horizontal scrolling,
overscroll, animation, or native accessibility adapters.
