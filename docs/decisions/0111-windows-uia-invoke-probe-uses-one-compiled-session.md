# Decision 0111: Windows UI Automation Invoke probe uses one compiled session

**Status:** Accepted

**Date:** 2026-08-24

## Context

Decision 0069 makes `IInvokeProvider` available only to a visible enabled button
in an authenticated UI session. Its unit and host tests prove the mailbox rule,
but they cannot prove that a real Windows client acquires the client-side
`IUIAutomationInvokePattern`, invokes it, and causes the existing authenticated
`ui.events.read` route to deliver the matching action to a child.

The compiled native UI diagnostic already has exactly that deterministic shape:
it sends one immutable document with one enabled
`native.ui.complete` button, waits only for that revision-bound action, and
requests session close only after it receives it. Reusing it tests the real
application boundary without a Node.js runtime, a browser, a test-only
application protocol, or a second action mailbox.

## Decision

Add a development-only `--uia-invoke-probe <native-client.exe>` route. It
launches the selected compiled native UI diagnostic through the existing fixed
three-grant development session. Once its host-created session window is
visible, a private MTA worker waits for only the compiled
`native.ui.complete` control, requires its standard Invoke pattern, and calls
`IUIAutomationInvokePattern::Invoke` once.

The probe passes only when the normal child exits successfully after its
existing `ui.events.read` and `session.close` sequence. It accepts no document,
button ID, UI Automation pattern, coordinate, action value, focus target, or
application result. On a probe failure, its worker requests closure of only
that known development session; the host terminates its selected child before
returning the fixed failure category. No Invoke interface, candidate, event,
or observer result crosses Anodrel's protocol or SDK boundary.

## Consequences

Positive:

- a real Windows UI Automation client verifies the provider's positive Invoke
  pattern path and the existing authenticated semantic-action delivery end to
  end; and
- the fixed UI Lab can remain correctly non-invokable under Decision 0110.

Tradeoffs:

- the route proves one compiled development button, not caller-selected action
  control, disabled-button refusal, screen-reader speech, or application
  behavior beyond the child's fixed success sequence; and
- it remains a developer-selected executable route, never a package, signing,
  installation, or product-launch mechanism.

## Revisit conditions

Revisit before accepting a caller-selected UI Automation target or pattern,
exposing an Invoke result to an application, adding an automation event
subscription, changing the native diagnostic's grants, or adding a non-Windows
equivalent.
