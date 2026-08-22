# Decision 0097: Windows UI Automation scrolling remains host-owned

**Status:** Accepted

**Date:** 2026-08-21

**Extension:** Decision 0098 adds host-owned `IScrollItemProvider` for eligible
children of this same selected viewport. This record retains the direct
ScrollPattern boundary.

## Context

Decision 0096 gives the direct Windows UI Lab and authenticated session view one
visible, pointer-operable vertical scrollbar. Its position is still entirely
host-retained: wheel input, Page Up/Page Down, track paging, and thumb dragging
change no document field, protocol value, application event, or focus target.

That surface is not yet equally operable by a person using Windows UI
Automation. A screen reader or automation client needs the standard
`IScrollProvider` control pattern, but a provider method can arrive on a thread
that does not own the custom-drawn view. Letting that method touch a native view
or making scroll position application data would break the existing boundary.

The current input policy deliberately selects only the first visible overflowing
scroll metric in source order. Nested scrolling, horizontal movement, and
scroll-into-view semantics have not been designed.

## Decision

The first visible overflowing `Scroll` node keeps its mapped `Group` control
type and additionally exposes the Windows UI Automation `ScrollPattern`
(`IScrollProvider`). No root, non-scroll group, static text, field, button,
diagnostic-free window, non-overflowing viewport, or later nested viewport
exposes the pattern.

The provider publishes one immutable scroll snapshot: the selected element ID,
its finite viewport and content heights, and the retained vertical offset that
produced the same layout. It reports:

- vertical scrolling only;
- no horizontal movement or horizontal view;
- the standard no-scroll percent for the horizontal axis;
- vertical percent in the closed range 0 through 100; and
- vertical view size as the finite current viewport/content percentage.

`Scroll` accepts only no horizontal movement plus one of the standard vertical
small or large increments. It maps small increments to the existing local line
operation and large increments to the existing local page operation.
`SetScrollPercent` accepts only horizontal no-scroll plus one finite vertical
percentage from 0 through 100, mapped to the existing clamped absolute offset.
Unsupported directions, malformed values, unavailable views, stale providers,
occupied routes, failed wakeups, timeouts, and host refusal have no new
application-visible distinction.

Each provider receives only a revision-bound, one-request scroll route. The
route carries a semantic viewport ID and a closed host command, never a native
handle, pointer data, layout, document, application callback, or registry
entry. It waits at most 250 ms while a fixed payload-free private window message
asks the owner UI thread to inspect the route. The UI thread revalidates the
provider revision and confirms that the same viewport is still the first
overflowing metric before changing its own `UiScrollState`. A timed-out route is
released before a late UI thread can apply its command.

The new scroll position remains host-local. It does not alter a document
revision, focus, field state, semantic-action queue, protocol, capability,
installed record, or application result. This slice does not add automation
scroll events, nested arbitration, horizontal scrolling, touch/gesture input,
application styling, persistence, or a non-Windows adapter. Decision 0098
separately adds the companion no-alignment `IScrollItemProvider` route without
widening any of those boundaries.

## Consequences

- Windows accessibility clients can operate the same vertical surface a pointer,
  wheel, and keyboard already operate.
- The provider reports only values calculated from the exact immutable layout
  snapshot it publishes; it never reads a mutable view during an automation
  query.
- The UI thread is the exclusive writer of retained scrolling state and can
  refuse a stale or no-longer-overflowing target before it changes anything.
- Applications cannot learn that a screen reader requested movement or infer a
  user's position from any protocol output.

## Revisit conditions

Revisit before adding scroll-position events, horizontal movement, another or
nested automation target, touch or kinetic movement, application-facing scroll
state or styling, a different UI Automation pattern, a native scrollbar
control, or another operating-system adapter.
