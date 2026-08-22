# Decision 0096: Windows scrollbar input stays host-owned

**Status:** Accepted

**Date:** 2026-08-21

## Context

The owned v2 scroll document model already measures a vertical viewport,
retains its offset in the host, clips its child, and accepts local wheel and
Page Up/Page Down movement. A long document nevertheless has no visible
position affordance and no direct pointer route to a position. Delegating that
to an application would require raw pointer messages, a scroll position, or
scroll events to cross the protocol, all of which expose behaviour rather than
application content.

The model permits nested viewports, but the current host's wheel and page
policy deliberately targets the first visible metric. Simultaneously drawing
overlapping controls for nested viewports would add target arbitration without
establishing an accessibility or input rule for it.

## Decision

The direct Windows UI Lab and authenticated UI-session view render one
host-owned vertical scrollbar for the first visible overflowing scroll metric
in source order. The scrollbar is an overlay at the viewport's trailing edge:
it does not alter the portable document's measurement, node tree, field
values, focus traversal, accessibility snapshot, or interchange format.

The host derives the track and thumb solely from the current layout metric and
the retained `UiScrollState`. The thumb has a finite minimum visual length, is
clamped inside its finite track, and maps exactly to the existing bounded
absolute scroll state. A completed pointer click on the track pages one current
viewport toward the clicked side. A press on the thumb starts host-local pointer
capture; subsequent pointer movement changes only that retained scroll offset,
and a button release stops the drag. A scrollbar interaction never focuses an
application element, invokes an action, edits a field, emits an application
event, or changes a document revision.

The scrollbar's geometry, hit testing, paging direction, and drag mapping live
in one pure Windows-host module with unit tests. The UI Lab converts current
client coordinates to logical coordinates before calling it. Win32 pointer
capture is used only while a host-owned thumb drag is active; the captured
pointer is not sent to the application.

The initial slice does not add horizontal scrollbars, arrow buttons, inertial
or gesture input, nested-scrollbar arbitration, scrollbar theming from an
application, a scroll position protocol field, scroll event, persistence,
native handle, or UI Automation scroll pattern. Existing keyboard and wheel
routes remain host-local. A separately designed accessibility scrolling surface
is required before assistive technology can control the position directly.

## Consequences

- A person can see whether the first visible viewport has more content and can
  reach its position directly with a pointer.
- The layout and protocol contracts remain unchanged; a document cannot
  customise or observe its scrollbar.
- The established first-viewport input policy remains explicit instead of
  pretending nested scrollbars have a solved pointer or accessibility rule.
- The direct renderer stays first-party and uses no browser, webview, toolkit,
  or third-party UI runtime.

## Revisit conditions

Revisit before adding a second rendered scrollbar, nested target arbitration,
horizontal movement, arrow buttons, touch or gesture input, kinetic animation,
application-facing scroll positions or events, styling input, persistence,
UI Automation `ScrollPattern`, a native scrollbar control, or another
operating-system adapter.
