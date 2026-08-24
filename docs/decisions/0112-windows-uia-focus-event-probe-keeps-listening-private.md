# Decision 0112: Windows UI Automation focus-event probe keeps listening private

**Status:** Accepted

**Date:** 2026-08-24

## Context

Decision 0074 defines an outbound `UIA_AutomationFocusChangedEventId` for a
genuine host-owned focus transition. Unit tests prove its emission gate, and
the existing focus probe proves that a real Windows client can call `SetFocus`
and observe the resulting focus. Neither proves that Windows delivers the
outbound event to a UI Automation client registered before that transition.

The existing fixed UI Lab supplies one deterministic event target:
`ui.lab.field`. It is a visible, enabled, keyboard-focusable field with no
application session or person-supplied document. It can therefore exercise a
real listener without creating an application-visible accessibility
subscription or a general event-client interface.

## Decision

Add a separate development-only `--uia-focus-event-probe` route with one private MTA
`IUIAutomationFocusChangedEventHandler`. Before calling `SetFocus` for the
compiled `ui.lab.field`, the worker registers the handler with Windows,
arms it once, and waits only for an event whose sender has that exact compiled
AutomationId. The existing `--uia-focus-probe` keeps its separate
`UIA_HasKeyboardFocusPropertyId` check, so each route has one direct Windows
assertion.

The handler copies only the event sender's AutomationId into one bounded
private result slot while Windows lends the sender to the callback. It holds no
application input, window handle, document, provider pointer, callback,
listener state, or result surface. It unregisters and releases its COM handler
before the worker reports its fixed outcome. The production host continues to
retain no subscribers and never checks whether assistive technology is
listening.

## Consequences

Positive:

- one first-party direct Windows client verifies the full focus-event delivery
  route, including the provider's outbound call and Windows' client callback;
- the existing fixed-focus acceptance route remains deterministic and free of
  application data; and
- the test handler's COM lifetime is paired with explicit unregister and
  release operations.

Tradeoffs:

- the direct client needs a small hand-written COM callback and the exact
  Windows event-registration vtable slots; and
- the check proves one changed fixed focus target, not narrator speech,
  keyboard input, disabled or clipped refusal, arbitrary event subscriptions,
  or application behaviour.

## Revisit conditions

Revisit before exposing any event listener, event sender, handler result,
selector, focus target, callback, or assistive-technology presence to an
application; adding a different event kind; or adding a non-Windows event
probe.
