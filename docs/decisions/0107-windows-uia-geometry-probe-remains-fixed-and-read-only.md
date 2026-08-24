# Decision 0107: Windows UI Automation geometry probing remains fixed and read-only

**Status:** Accepted

**Date:** 2026-08-24

## Context

Decision 0106 made the fixed UI Lab's UI Automation properties and trees
repeatable through a direct first-party Windows client. That proves the host
publishes the expected nodes in raw and control views, but it does not prove a
published bounding rectangle lies on the window or that Windows resolves a
point inside it to the same semantic node.

Inspect and Accessibility Insights remain the right tools to confirm a visual
highlight looks correct, and Narrator remains the only evidence of spoken
output. A small deterministic geometry check can still catch a class of broken
provider geometry without turning the diagnostic into an arbitrary UI
automation client.

## Decision

Extend the fixed host-only UI Automation probe with exactly one geometry target:
the compiled UI Lab's visible `ui.lab.field` Edit. The direct client reads the
host root's and field's current bounding rectangles, verifies that both are
non-empty and that the field rectangle lies within the root, then asks Windows
for the element at the field rectangle's centre. A passing result requires that
element to report the same fixed AutomationId.

The point is derived entirely from the current host-published rectangle. The
operator supplies no coordinate, selector, document, window handle, or input.
The client does not use ClickablePoint, click, Invoke, SetFocus, Scroll,
SetValue, or event registration. The application receives no geometry, hit
result, listener state, or probe result, and the protocol, SDK, document
format, capability list, installed record, and product session do not change.

`ElementFromPoint` asks Windows for the topmost desktop element, so the
short-lived private UI Lab diagnostic places only its own test window in the
topmost band before the query. That protects the fixed check from an ordinary
editor or terminal covering its field. The window is destroyed immediately
afterward; no product window is made topmost, and a higher-priority Windows
surface can still cause the probe to fail rather than create a false pass.

## Consequences

Positive:

- a malformed or off-window field rectangle is caught by a repeatable
  real-Windows check;
- the provider's own fragment hit testing is exercised through the Windows
  client boundary; and
- no browser, webview, test framework, or third-party UI Automation binding is
  introduced.

Tradeoffs:

- it covers one fixed visible target, not arbitrary document geometry or a
  person-visible highlight; and
- the intentionally run diagnostic briefly changes the z-order of its own
  temporary window so the desktop-level query has a meaningful subject; and
- Windows composition and screen-reader speech remain manual acceptance work.

## Revisit conditions

Revisit before adding another target, a caller-supplied point or selector,
ClickablePoint, an interactive pattern, a UI Automation event, arbitrary
window selection, application-visible geometry, or a non-Windows equivalent.
