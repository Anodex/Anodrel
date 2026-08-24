# Decision 0113: Windows UI Automation focus probe reads provider focus

**Status:** Accepted

**Date:** 2026-08-24

## Context

Decision 0109 originally made the fixed focus probe call
`IUIAutomation::GetFocusedElement` after `SetFocus`. The direct Windows
focus-event probe in Decision 0112 proves the host's semantic focus transition
does occur and reaches a registered UI Automation client. On the same
custom-drawn Anodrel surface, however, Windows' global focused-element query
can report the native window root rather than the child fragment whose
provider-owned semantic focus changed.

That global query represents Windows input focus. Anodrel deliberately keeps
semantic accessibility focus separate from native foreground activation and
does not make a child fragment into a native HWND. Treating the global result
as the source of truth would make a passing test depend on unrelated desktop
focus policy instead of the provider contract it is intended to verify.

## Decision

The fixed `--uia-focus-probe` still finds and calls `SetFocus` on only the
compiled `ui.lab.field`. It then requests a fresh provider publication for the
same temporary window, finds that same fixed field again, and passes only when
Windows reads `UIA_HasKeyboardFocusPropertyId` as true on the fresh element.

The probe no longer calls `GetFocusedElement`. It does not request native
foreground activation, read a window handle, expose focused identity, accept a
selector, or make focus observable to an application. The normal direct
Windows focus-event probe remains a separate test of outbound event delivery.

## Consequences

Positive:

- the fixed query probe proves the provider's actual semantic focus state;
- the result is independent of desktop foreground policy and unrelated native
  input focus; and
- the event probe and property probe now cover distinct, directly relevant
  boundaries.

Tradeoffs:

- this is not a test of Windows' global focused-element API; and
- full Narrator and Inspect verification remain separate manual checks.

## Revisit conditions

Revisit before exposing semantic focus to applications, joining it to native
foreground activation, adding caller-selected targets, or adding a
non-Windows focus probe.
