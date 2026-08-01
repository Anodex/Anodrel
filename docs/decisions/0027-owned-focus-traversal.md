# Decision 0027: Native UI focus starts as owned layout-bound traversal

**Status:** Accepted

**Date:** 2026-08-01

## Context

Keyboard interaction and assistive technology require a stable notion of which
control is currently selected. Delegating this to one host renderer would make
navigation order inconsistent, while exposing raw operating-system focus before
the portable UI model is proven would couple permissions, lifecycle, and input
delivery to platform-global state.

## Decision

`anodrel-ui` owns a small `UiFocus` value. Against one concrete layout, it
traverses only visible enabled actions in document source order, wraps at either
end, and can turn a still-valid focus target into the existing semantic action
event. The complete behavior is documented in `docs/UI.md`.

The value does not receive keyboard messages, set native focus, render a focus
indicator, edit text, or invoke a native operation. A Windows, Linux, or macOS
adapter must explicitly decide how input, accessibility focus, repaint, window
activation, and application-session delivery map to it.

## Consequences

- Keyboard order is deterministic and matches the document rather than host
  widget ordering;
- focus never lands on disabled or invisible content; and
- action activation stays semantic and cannot become ambient authority.

## Revisit conditions

Revisit before accepting untrusted UI documents, adding editable controls,
pointer capture, focus notifications, operating-system focus or accessibility
integration, or delivering action events to an application session.
