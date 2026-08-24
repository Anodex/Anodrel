# Decision 0110: Windows UI Automation probe keeps UI Lab buttons non-invokable

**Status:** Accepted

**Date:** 2026-08-24

## Context

The fixed UI Lab deliberately displays buttons so rendering, pointer input, and
semantic layout can be exercised. It has no authenticated application event
mailbox. Decision 0069 therefore confines the UI Automation `Invoke` pattern
to enabled buttons in authenticated UI sessions; exposing it on one of the
Lab's diagnostic buttons would make an accessibility client appear to have an
application action route that does not exist.

Provider unit tests enforce that rule, but the existing real-Windows property
probe reads only node properties, geometry, and the fixed field's Value pattern.
An accidental COM registration or client/provider mismatch could still publish
`Invoke` on a Lab button without changing those values.

## Decision

Extend the existing read-only `--uia-property-probe` with one presence check:
for every Anodrel semantic node in the fixed UI Lab, its direct Windows UI
Automation client calls `IUIAutomationElement::GetCurrentPattern` for
`UIA_InvokePatternId` and requires no returned pattern interface.

The client does not call `IUIAutomationInvokePattern::Invoke`, accept a node,
pattern, window, document, action, coordinate, or input, or report a result to
an application. It releases any interface Windows returns before the temporary
diagnostic window closes. The rule does not restrict the host window's normal
Windows-provided Window or Transform patterns, and it does not alter
authenticated-session Invoke behavior defined by Decision 0069.

## Consequences

Positive:

- a real Windows client now guards the boundary between the UI Lab's local
  diagnostic buttons and authenticated semantic actions; and
- the existing property probe remains a fixed, host-only, non-interactive
  acceptance diagnostic.

Tradeoffs:

- this confirms only absence on the compiled UI Lab, not Invoke delivery on an
  authenticated session; and
- activation, event delivery, application actions, and caller-selected pattern
  queries remain outside the probe and require their own verification.

## Revisit conditions

Revisit before exposing any UI Lab action to a protocol client, adding another
pattern check, accepting a caller-selected pattern or node, or adding a
non-Windows equivalent.
