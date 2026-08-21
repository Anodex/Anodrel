# Decision 0076: UI Automation structure events follow accepted document replacement

**Status:** Accepted

**Date:** 2026-08-21

## Context

An authenticated Anodrel UI session can replace its complete validated document
through its existing revision-bound mailbox. A later UI Automation query sees
the new immutable tree, but existing clients are not told that the child subtree
was replaced.

The standard `ChildrenInvalidated` kind describes a whole substituted subtree.
The notification must not become a listener registry, a callback, an
application-visible event stream, or an event for every paint and text edit.

## Decision

Only after the UI thread accepts and applies a strictly newer authenticated
session document, it builds one fresh publication and raises
`StructureChangeType_ChildrenInvalidated` through
`UiaRaiseStructureChangedEvent` on the window fragment root. The runtime-ID
pointer is null and its length is zero, as Windows uses it only for
`ChildRemoved`.

No event is raised for an empty, stale, or rejected mailbox snapshot; local
paint, layout, resize, typing, field-value change, focus, action, dialog,
notification, closure, or diagnostic/preview surface. The call occurs after the
view-registry lock is released. It is best effort: no listener check, retained
subscriber, HRESULT reporting, log, protocol field, capability, or application
callback exists.

## Consequences

- Assistive technology can refresh a replaced authenticated document subtree.
- Complete replacement reports one honest invalidation, not an invented list of
  additions and removals.
- Document validation, revision checks, and session capability boundaries stay
  unchanged.

## Revisit conditions

Revisit before property/value/text events, detailed child-diff events, live
regions, listener tracking, application-visible event status, or non-Windows
accessibility event adapters.
