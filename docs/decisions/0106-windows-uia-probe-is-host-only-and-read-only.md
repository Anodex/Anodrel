# Decision 0106: Windows UI Automation probing is host-only and read-only

**Status:** Accepted

**Date:** 2026-08-24

## Context

Anodrel's Windows UI Automation provider has focused unit and host tests, but
several acceptance checks still require Inspect, Accessibility Insights, or a
screen reader. Those tools remain necessary for spoken behaviour and visual
highlighting, yet a large part of their property and tree inspection can be
made repeatable.

The provider must not gain a feedback path merely to test itself. In
particular, applications must not learn whether a UI Automation client is
connected, read an automation tree, select a target, or receive a probe result.
Adding a browser test runner or a third-party UI Automation binding would also
contradict Anodrel's native-first runtime boundary.

## Decision

Add a development-only, host-local `--uia-property-probe` route. It opens the
fixed UI Lab, then a separate host thread initializes a direct Windows UI
Automation client and queries only that window's published immutable tree.
The probe uses only `Ole32`, `OleAut32`, and Windows UI Automation APIs.

Its first slice is read-only. It obtains the host window's automation root,
walks its raw child tree and its control-view tree, and reads a closed set of
published properties:

- `Name`;
- `AutomationId`;
- `ControlType`; and
- structural child/sibling order.

The host compares those values with the fixed UI Lab contract in each view,
writes one fixed pass/fail result to its operator console, and closes its own
diagnostic window. The normal framed Windows window contributes one native
`TitleBar` peer before the Anodrel semantic viewport; the probe verifies that
peer and then verifies the Anodrel subtree separately. The control-view check
is meaningful because the provider declares every bounded semantic node to be
a control element, but it remains a tree check rather than a claim about
spoken output. No application is connected to this route, and nothing is added
to the protocol, SDK, document format, capability list, installed record, or
product session.

The probe does not invoke a button, set focus, edit a field, scroll, subscribe
to events, inspect assistive-technology presence, or report arbitrary returned
text. Those surfaces remain separate manual or future automated checks because
they have their own authority and lifetime contracts.

## Consequences

Positive:

- provider property and hierarchy regressions become repeatable Windows
  verification using only first-party code and direct OS APIs;
- the host can verify the COM client/provider boundary that pure mapping tests
  cannot exercise; and
- Inspect and Narrator can focus on their irreplaceable visual and spoken
  acceptance checks.

Tradeoffs:

- the probe is Windows-only and intentionally tied to a fixed diagnostic
  document; it is not a general application automation API;
- direct COM client ABI declarations need focused review and real-Windows
  verification; and
- passing this probe does not claim a screen reader announced, highlighted, or
  interacted with a surface correctly.

## Revisit conditions

Revisit before adding a query for live field state, focus control, Invoke,
Scroll, any UI Automation event handler, an arbitrary window selector, an
application-visible result, or a non-Windows equivalent.
