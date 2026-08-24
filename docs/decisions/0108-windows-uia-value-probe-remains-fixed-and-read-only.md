# Decision 0108: Windows UI Automation value probing remains fixed and read-only

**Status:** Accepted

**Date:** 2026-08-24

## Context

Decision 0071 defines a read-only `IValueProvider` for a visible Anodrel field.
Its unit tests verify the provider object and its refusal to write, but the
repeatable real-Windows probe currently verifies only tree properties and one
field rectangle/hit target. A client can reach a field in the tree while the
pattern interface it relies on is absent, misbound, or returns the wrong
ownership form.

The compiled UI Lab has exactly one visible field, `ui.lab.field`, whose initial
value is the fixed empty string. It is therefore an appropriate deterministic
pattern target without reading a person's data or accepting a caller-selected
field.

## Decision

Extend the existing host-only UI Automation probe with one fixed Value-pattern
read. Anodrel publishes the provider-side `IValueProvider`; the direct Windows
client obtains the corresponding client-side `IUIAutomationValuePattern` from
`ui.lab.field`, reads its current value and `IsReadOnly` flag, and passes only
when the value is the compiled empty string and the flag is true.

The client does not call `SetValue`, focus, invoke, scroll, click, register an
event, or accept a pattern, field, document, window, coordinate, or value from
an operator or application. The BSTR returned by Windows is copied into the
private probe result and released before the worker ends. No field value,
pattern interface, listener state, or result reaches the protocol, SDK,
installed record, product session, or application.

## Consequences

Positive:

- a real Windows UI Automation client proves the field exposes the correct
  read-only Value pattern, not just a matching `Edit` control type;
- the test covers normal BSTR ownership and the pattern's separate read-only
  flag through the OS boundary; and
- no browser, webview, test framework, or third-party binding is introduced.

Tradeoffs:

- the probe confirms one compiled empty value, not a later person-entered
  value, field replacement, or application-facing field read; and
- `SetValue`, selection, caret, text ranges, value events, and every write
  route remain absent and separately deferred by Decision 0071.

## Revisit conditions

Revisit before adding a caller-selected field, non-fixed value, `SetValue`, text
or selection patterns, a value-change event, application-visible accessibility
data, or a non-Windows equivalent.
