# Decision 0037: Scroll state starts as an owned bounded primitive

**Status:** Accepted

**Date:** 2026-08-01

## Context

Long native application documents need scrolling, but adding a permissive
scrolling node or binding raw wheel messages directly into application content
would blur the model, renderer, and host-input boundaries.

## Decision

`anodrel-ui` first exposes `UiScrollState`: a small owned vertical offset that
accepts line, page, and absolute movement only after finite clamping against
caller-supplied content and viewport extents. It identifies no document or
element, stores no input event, and has no renderer, callback, protocol, or
operating-system dependency.

## Consequences

- future native hosts share the same scroll clamping behavior;
- resizing cannot leave a stale out-of-range offset; and
- scroll containers, wheel/key routing, accessibility position, and a new
  strict document format remain explicit follow-up work.

## Revisit conditions

Revisit before adding a scroll node, horizontal scrolling, inertia, overscroll,
scrollbars, application-facing scroll events, or any external format field.
