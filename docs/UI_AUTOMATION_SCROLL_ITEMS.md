# Anodrel Windows UI Automation scroll-item contract

**Status:** Implemented and covered by focused portable, provider, and
Windows-host tests. Manual Narrator and Inspect verification remains required.

## Purpose

`IScrollItemProvider::ScrollIntoView` lets a Windows accessibility client ask a
scroll container to reveal one of its children. It complements, but does not
extend, Anodrel's host-owned vertical `IScrollProvider` contract in
`docs/UI_AUTOMATION_SCROLL.md`.

The operation reveals a semantic element through a viewport Anodrel already
owns. It does not let an application choose a scroll position or learn that a
person or automation client requested movement.

## Published items

An accessibility snapshot contains every node in the current bounded document
preorder. Its `BoundingRectangle` is its clipped current rectangle; a node
wholly outside every ancestor clip has an empty rectangle and
`IsOffscreen=true`.

Only the first **visible** overflowing scroll viewport in source order can
publish scroll items. The viewport itself keeps `ScrollPattern`; each of its
published descendants whose nearest semantic scroll ancestor is that viewport
also publishes `ScrollItemPattern` / `IScrollItemProvider`.

This means a client can reach a fully clipped child in the immutable navigation
tree and ask the outer viewport to reveal it. A child inside a nested scroll
viewport is deliberately not an item of the outer viewport. A nested viewport
itself can be revealed as an outer item, but its own contents need a later
nested-arbitration decision.

No pattern is published for the window root, the selected viewport itself,
unrelated elements, a non-overflowing viewport, a later viewport, or a
provider without the host route.

## Operation

`ScrollIntoView()` has no parameter and gives no alignment option. The host
uses the current layout:

- a smaller item above the viewport moves to the top edge;
- a smaller item below it moves to the bottom edge;
- an item taller than the viewport moves to the top edge; and
- an item already wholly visible is accepted with no movement.

The existing finite `UiScrollState` clamp sets the final position. A client
receives generic success or failure only; it cannot learn why a request was
refused or what offset resulted.

## Thread and authority boundary

The COM method may run away from the window's UI thread. It writes one
revision-bound request into the existing one-slot scroll mailbox, wakes the
owner with the same payload-free private window message, and waits at most
250 ms.

The request carries only the selected semantic viewport ID and the selected
semantic item ID. The owner accepts it only after confirming the revision, the
current first visible overflowing viewport, the item's nearest scroll ancestor,
and current geometry. No COM method reads a mutable view or registry.

No direction crosses into application authority:

- applications cannot create an item pattern, choose a viewport or alignment,
  receive a scroll event, read a position, or detect assistive technology;
- automation clients receive no native handle, layout, document, pointer data,
  field text, application callback, or refusal detail; and
- revealing an item changes no protocol field, grant, document revision,
  action, focus, field state, or automation event.

## Interaction with visibility

An empty clipped rectangle makes a node non-interactive to local input and to
the existing visible-only UI Automation patterns. It therefore cannot be
focused, invoked, or expose a copied field value until a fresh publication
shows it in view. `IScrollItemProvider` is the one narrow exception: it exists
to make that visibility change possible.

The software renderer does not paint a fully clipped layout item. Keeping its
semantic record for accessibility is bounded by the document's 512-node limit
and does not add raster work for off-screen content.

## Explicitly deferred

Nested scroll-item routing, horizontal movement, arbitrary alignment,
automatic focus after reveal, `IScrollItemProvider` events, virtualized item
providers, scroll-position events, application-owned scroll state, persistence,
and non-Windows adapters remain outside this contract.

## Verification plan

Automated coverage must prove:

- an off-screen descendant remains in the semantic tree with an empty rectangle;
- visible-only input, focus, Invoke, and Value rules still refuse that node;
- only the selected viewport's eligible nearest descendants expose the
  interface and pattern;
- a command carries both the immutable viewport and item identities;
- above, below, oversized, already-visible, stale, changed, nested, busy, and
  timed-out cases fail closed or use the documented alignment; and
- the owner changes the same retained scroll state used by pointer, wheel, and
  keyboard scrolling.

After automated checks pass, Narrator and Inspect or Accessibility Insights
must verify that an off-screen item appears in the tree, exposes
`ScrollItemPattern`, becomes visible after `ScrollIntoView`, and never receives
an unintended focus or action.
