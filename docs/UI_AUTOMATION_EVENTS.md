# Anodrel UI Automation events

**Status:** Implemented on Windows; manual UI Automation-client verification is
still required.**

## Purpose

Anodrel’s Windows UI Automation provider can report and move the host-owned
keyboard focus. A client that already holds a provider can read its copied
result, but a screen reader also needs an ordinary UI Automation notification
when a person moves focus with a pointer or keyboard, or a UI Automation client
successfully moves it through the bounded route.

This document defines the first event only: `UIA_AutomationFocusChangedEventId`
(`20005`). It is an outbound host-to-Windows signal, not an application event
system. See Decisions 0074, 0073, and 0070.

## Exact event

The Windows host raises `UIA_AutomationFocusChangedEventId` only after its own
`UiFocus` changes to a visible, enabled, keyboard-focusable field or button.
The event source is a fresh, immutable provider for that post-change target and
the same current layout publication the host would answer through
`WM_GETOBJECT`.

The host raises no event when:

- a pointer or keyboard operation leaves focus unchanged;
- a `SetFocus` request is refused, stale, timed out, or targets the element
  already focused;
- a UI surface has no current published focus target; or
- a view is unavailable or closes before the host can build the publication.

The event API’s return value is intentionally discarded. It is not logged,
sent to an application, used as a listener check, or allowed to alter host
focus. A failure can mean only that Windows did not accept this best-effort
outbound notification; it never changes the completed local focus operation.

## Lifetime and threading

Focus changes happen only on the window’s owner UI thread. Once the registry
mutation is complete and its lock is released, that same thread derives a fresh
accessibility publication and calls `UiaRaiseAutomationEvent` with the focused
child provider. UI Automation retains its own reference while handlers process
the event; Anodrel releases only its creation reference after the call.

No provider pointer, tree, registry entry, native handle, callback, or event
result reaches an application. The host never stores event subscribers and does
not call `UiaClientsAreListening`: listener presence must remain unobservable,
and focus changes are infrequent enough that skipping the check is the simpler
and safer behaviour.

## Boundaries

- No protocol version, operation, grant, SDK method, installed-record field,
  or application callback is added.
- The event does not activate a window, send input, invoke a button, edit a
  field, or reveal the focused element to an application.
- It contains no application-supplied event text, data, or identifier beyond
  the ordinary immutable provider values already visible to Windows.
- There are no Invoke, property-change, value-change, text, live-region,
  notification, structure, or selection events in this slice. Each needs its
  own contract and decision.

## Verification

Automated checks must prove that the host distinguishes an accepted `SetFocus`
no-op from a changed focus target and only asks the event adapter to raise an
event for the latter. Adapter tests must prove it refuses an empty publication
without calling Windows and uses the published focus child as the event source.

Manual Windows verification uses a UI Automation client registered for
focus-changed events while an Anodrel UI Lab or UI Session Lab is open. Tab,
pointer focus, and a successful UI Automation `SetFocus` must each produce one
event naming the new visible control. Repeating focus on the same control and
attempting a disabled or clipped target must produce none. This check is
required before the feature is called manually verified.
