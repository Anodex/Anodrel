# Decision 0098: Windows UI Automation scroll items remain host-owned

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0097 lets Windows UI Automation operate the first visible overflowing
Anodrel viewport through `IScrollProvider`. That is sufficient for direct line,
page, and percentage movement, but not for ordinary screen-reader navigation:
an automation client needs to ask the viewport to reveal a particular child.
Windows expresses that request through `IScrollItemProvider::ScrollIntoView`.

The old accessibility snapshot omitted a node once every pixel of it was
clipped. That is correct for hit testing, keyboard focus, painting, and the
existing visible-only patterns, but it would make a ScrollItem pattern
meaningless: the automation tree could never contain the child below the
viewport that needed revealing.

The solution must not turn host-retained position into document data,
application input, focus, an event stream, or a general geometry API. A UI
Automation call can also arrive away from the window's UI thread, so it cannot
touch the custom view directly.

## Decision

One bounded `UiLayout` now retains every semantic node in source order, even
when its clipped bounds are empty. The portable accessibility snapshot preserves
that complete bounded preorder and direct semantic parentage. An empty clipped
rectangle remains the only geometry published for a wholly clipped node, and
the Windows adapter reports `IsOffscreen=true` for it.
Painting, pointer hit testing, keyboard focus, button invocation, and field
value publication continue to require non-empty clipped bounds; retaining a
node for navigation does not make it locally interactive.

For a publication whose first visible overflowing viewport exposes
`IScrollProvider`, every published descendant whose **nearest** semantic scroll
ancestor is that viewport exposes `IScrollItemProvider`. The viewport itself,
the window root, unrelated nodes, non-overflowing viewports, and descendants
inside a nested scroll viewport do not. A nested viewport may itself be an item
of its selected outer viewport, but its contents remain deferred with nested
scroll arbitration.

`ScrollIntoView` offers one closed command through the same revision-bound,
single-slot, 250 ms private scroll route as Decision 0097. The command contains
only the selected viewport ID and the semantic item ID. On its owning UI
thread, the host confirms the provider revision, confirms that viewport is
still the first visible overflowing metric, confirms that the item remains one
of its permitted nearest descendants, and derives geometry from the current
layout. It then changes only the existing `UiScrollState`:

- an item above the viewport aligns its top with the viewport's top;
- an item below the viewport aligns its bottom with the viewport's bottom;
- an item taller than the viewport aligns its top; and
- an already fully visible item succeeds without changing position.

The normal scroll-state clamp remains the final authority. There is no caller
control over alignment, offset, another viewport, a native handle, or pointer
data.

No application observes this path. It adds no protocol field, capability,
document value, revision, semantic action, focus move, field change,
notification, application callback, scroll-position readback, or automation
event. A provider is still immutable: only a fresh `WM_GETOBJECT` publication
reflects a later retained offset.

## Consequences

- A screen reader can navigate to a bounded off-screen descendant and request
  that the same host-owned viewport reveal it.
- The accessibility tree describes all declared bounded content while its
  rectangles remain truthful about what is currently visible.
- Existing visible-only interaction rules stay intact, avoiding an off-screen
  Invoke, value read, or focus path.
- The added layout work is bounded by the existing 512-node document maximum;
  the renderer skips fully clipped nodes before drawing them.

## Revisit conditions

Revisit before adding nested-scroll item routing, horizontal scroll items,
automation scrolling or structure events, automatic focus after reveal,
virtualized collections, scroll-position persistence, application-visible
scroll state, a different UI Automation pattern, or a non-Windows adapter.
