# Decision 0228: Fixed Value write rejection stays host-only

**Status:** Accepted

**Date:** 2026-09-19

## Context

Decision 0071 requires Anodrel fields to be read-only to UI Automation, and
Decision 0108 added one fixed client read of the empty UI Lab field. Provider
tests already confirm its internal `SetValue` implementation returns
`UIA_E_NOTSUPPORTED`, but the direct client had not exercised the standard
read-only precondition. A real client could otherwise expose a writable-looking
pattern despite the provider-unit result.

The Windows client rejects `IUIAutomationValuePattern::SetValue` on a pattern
whose current read-only property is true with `UIA_E_INVALIDOPERATION`. Testing
that client-side result must not accept an application value, field, document,
or write route.

## Decision

Extend the existing fixed `--uia-property-probe`. After reading the compiled
empty field and `IsReadOnly=true`, its private MTA client allocates one empty
BSTR, calls `SetValue` only for that compiled field, requires
`UIA_E_INVALIDOPERATION`, and reads the same field again to require the
unchanged empty read-only snapshot.

The client accepts no value, selector, field, document, window, callback, or
result from an application or operator. The provider's direct
`UIA_E_NOTSUPPORTED` unit assertion remains separate: the real client rejects
the call before it dispatches a write to a read-only pattern.

## Consequences

- The repeatable property probe now proves the real Windows client enforces
  the read-only boundary without creating a write capability.
- The existing direct-client probe remains eight fixed probes; this is a
  stronger assertion in its established fixed field check.
- Manual verification remains necessary for person-entered values, rendering,
  disabled fields, and spoken output.

## Revisit conditions

Revisit before accepting a non-empty or caller-selected value, field,
selector, document, window, callback, writable UI Automation pattern, caret,
selection, text range, value event, application-visible field observation, or
non-Windows equivalent.
