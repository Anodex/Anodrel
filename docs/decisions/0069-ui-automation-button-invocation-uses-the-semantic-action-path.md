# Decision 0069: UI Automation button invocation uses the existing semantic action path

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0063 and the Windows UI Automation provider made an Anodrel surface
readable. A person using Narrator can now find an enabled semantic button, but
cannot activate it. Reading and activation are different responsibilities: a
provider must not turn a screen-reader command into an unbounded native
operation, a pointer message, or a new application-facing bridge.

The authenticated UI session already has the one correct route for a person's
semantic action. The host derives `UiEvent::ActionInvoked` from its current
layout, binds it to that document's monotonic revision, places it in the
session's bounded input mailbox, and `ui.events.read` revalidates it before the
application receives it. That route rejects stale, removed, disabled, and
ungranted actions. It is the boundary UI Automation must join rather than
duplicate.

## Decision

Publish `IInvokeProvider` only for an **enabled `Button`** in an authenticated
UI session with a current document revision. The window root, text, groups,
fields, disabled buttons, and every diagnostic surface expose no Invoke
pattern.

Calling `Invoke` creates exactly one
`UiInputCandidate { revision, UiEvent::ActionInvoked(element_id) }` and offers
it to the same bounded `UiInputMailbox` that accepts native pointer and
keyboard activation. It does not send a Windows message, synthesize pointer or
keyboard input, move focus, update local field state, call an operating-system
API, or call application code. The candidate is still revalidated by the
existing `ui.events.read` path; an old provider can therefore never make a
replaced, removed, or disabled button reach application logic.

This changes no protocol field, capability grant, operation, or version. Invoke
is another host-owned producer for the existing semantic-action candidate;
applications continue to receive only the existing, revalidated
`ui.action.invoked` envelope after a separately granted `ui.events.read`.

The provider owns only a clone of the session's mailbox and the immutable
revision and element ID from the layout it published. It never retains the
window registry or a mutable view. It may therefore outlive `WM_GETOBJECT` or
the window safely: after the session closes, a queued candidate has no delivery
route, and a surviving session still applies its ordinary revision checks.

The mailbox remains bounded. A full queue makes `Invoke` fail with a generic
COM failure and creates no candidate; it does not grow a second queue or expose
queue state through UI Automation. Existing event delivery continues to report
its bounded dropped-action count only through the granted protocol operation.

## Consequences

Positive:

- a screen-reader user can activate the same enabled buttons as a pointer or
  keyboard user;
- there is one semantic action route, one revision rule, and one bounded queue
  to test and maintain; and
- accessibility adds neither native authority nor a signal that assistive
  technology is present.

Tradeoffs:

- fields remain discoverable but cannot be focused, edited, or have their value
  read through UI Automation; those are separate sensitive capabilities;
- diagnostic UI Lab actions remain local and have no authenticated-session mailbox,
  so they intentionally publish no Invoke pattern; and
- invocation still needs a real UI Automation client and a screen-reader check
  in addition to unit tests, because COM plumbing cannot prove a person can
  activate a control.

## Revisit conditions

Revisit when a semantic toggle, selection, value, scroll, text, focus, live
announcement, automation event, or a diagnostic action route is proposed. Each
needs its own contract, threat-model entry, and verification; none is implied
by Invoke.
