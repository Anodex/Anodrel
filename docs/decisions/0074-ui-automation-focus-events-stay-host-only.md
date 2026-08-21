# Decision 0074: UI Automation focus events stay host-only

**Status:** Accepted

**Date:** 2026-08-21

## Context

Anodrel's Windows UI Automation provider already reads the host's focus
snapshot (Decision 0070) and can move focus only through a bounded route owned
by the UI thread (Decision 0073). Those operations leave a screen reader with
one gap: it is not notified when a person moves focus locally. Polling copied
provider state is an unreliable substitute for the ordinary UI Automation
focus-changed signal.

Raising an event is not automatically safe. A provider must not retain mutable
view state, an event must not become an application callback or a signal that
assistive technology is listening, and a failed best-effort notification must
not change the result of keyboard, pointer, or automation focus.

## Decision

After `UiFocus` genuinely changes to a current visible enabled focus target,
the Windows host builds one fresh immutable provider publication and raises
`UIA_AutomationFocusChangedEventId` on that focused child through
`UiaRaiseAutomationEvent`.

The host does this only after it has released the view registry lock. It raises
no event for a failed request, unavailable view, stale provider, or a request
whose target was already focused. UI Automation owns a retained reference while
handlers process the event; Anodrel releases only the reference it created for
the call.

The host does not call `UiaClientsAreListening`, retain subscribers, report the
Windows result, log it, or deliver it through the protocol. The event is a
one-way, best-effort operating-system notification. It has no application
callback, operation, capability, version, native-input, activation, focus
readback, or listener-presence surface.

This decision adds no other event. Invoke, property/value/text changes,
structure, live announcements, and selection events remain separate decisions.

## Consequences

Positive:

- assistive technology can receive standard focus transitions for the same
  host-owned focus a keyboard and pointer use;
- the host has one exact emission condition, which can be tested independently
  of COM and Windows event delivery; and
- provider lifetime stays bounded to one outbound notification.

Tradeoffs:

- event delivery is best effort and not observable by an application;
- real UI Automation-client and screen-reader checks remain necessary; and
- no live provider, listener registry, or general event framework is created.

## Alternatives considered

**Poll focus from applications.** Rejected. It would expose personal navigation
and duplicate UI Automation's responsibility.

**Use `UiaClientsAreListening` before every event.** Rejected. It would create
a listener-presence branch on a sensitive boundary for a negligible focus-event
cost, and is not needed for correctness.

**Raise value, Invoke, or structure events at the same time.** Rejected. Each
has different data, authority, lifetime, and privacy implications.

**Retain one provider while a view lives.** Rejected. It would couple COM
lifetime to mutable registry state and invite stale-tree reads.

## Revisit conditions

Revisit before adding any event other than focus change, a listener tracking
mechanism, an application-visible focus surface, live-state provider lookup, or
a non-Windows accessibility event adapter.
