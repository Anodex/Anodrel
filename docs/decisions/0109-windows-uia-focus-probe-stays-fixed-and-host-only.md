# Decision 0109: Windows UI Automation focus probing stays fixed and host-only

**Status:** Superseded in part by Decision 0113

**Date:** 2026-08-24

## Context

Decision 0073 defines a bounded `IRawElementProviderFragment::SetFocus` route:
Windows may request focus for an already-published visible control, but only the
owning Windows UI thread may accept that request and update its local focus
state. Unit tests cover the route and its rejection conditions. They do not
prove that a real Windows UI Automation client can call `SetFocus` on an
Anodrel provider and then observe the selected element through Windows.

The compiled UI Lab has one deterministic focus target, `ui.lab.field`. It is
visible, enabled, and keyboard-focusable, with no person-provided document or
application session behind it. It is therefore suitable for a repeatable
acceptance check without creating a general UI Automation control client.

## Decision

Add a separate host-only `--uia-focus-probe` diagnostic. It opens the fixed UI
Lab, finds only `ui.lab.field` in its control view, calls the standard
client-side `IUIAutomationElement::SetFocus`, then reads
`IUIAutomation::GetFocusedElement`. The probe passes only when that returned
element has the same fixed automation ID. The temporary window closes before a
fixed success result is printed.

The probe accepts no window, selector, point, document, action, value, event,
or focus target from an operator or application. It does not inspect field
values, invoke a control, send synthetic input, request foreground activation,
subscribe to events, check for assistive technology, or disclose the focused
element. Its sole mutation is the already-defined host-owned focus transition
for the temporary fixed UI Lab window.

## Consequences

Positive:

- a real Windows UI Automation client verifies the complete `SetFocus` path,
  including the provider, private payload-free host route, UI-thread layout
  gate, and observable resulting focus; and
- the read-only property probe remains honestly read-only and can keep its
  narrow contract.

Tradeoffs:

- the check proves only one fixed visible target, not arbitrary client
  selection, focus events, keyboard input, or screen-reader speech; and
- Narrator and Inspect remain separate manual acceptance checks.

## Revisit conditions

Revisit before adding a caller-selected focus target, application focus
readback or control, automation input, a focus event subscription, a general
UI Automation client API, or a non-Windows focus probe.
