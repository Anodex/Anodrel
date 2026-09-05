# Decision 0202: Windows UI Automation live-status probe keeps listening private

**Status:** Accepted

**Date:** 2026-09-04

## Context

Decision 0100 permits one host-raised `UIA_LiveRegionChangedEventId` after a
later visible authenticated version-3 status changes. Unit checks prove the
status gate, provider source, and mapping, but cannot prove a real Windows UI
Automation client can register before the replacement and receive that event.

The production host must never retain a listener, ask whether anyone is
listening, or reveal event delivery to an application. A child-side readiness
message, selectable event, or acknowledgement would weaken that boundary.

## Decision

Add one development-only
`--uia-live-status-event-probe <native-client.exe>` route and one compiled
two-document native diagnostic. Its ordinary three-grant session first
publishes a fixed visible polite status with one fixed `prepare` action. A
private host MTA worker locates that action, registers one
`IUIAutomationEventHandler` for `UIA_LiveRegionChangedEventId` beneath the
fixed window root, arms it, then invokes that fixed action once.

The child publishes its fixed version-3 replacement with the same visible
status identity but an assertive changed value and one fixed `complete` action.
The probe passes only when Windows calls back with the exact live-event ID and
the fixed status AutomationId. It unregisters and releases its listener before
using a fresh private client to invoke `complete`, allowing the child to close
through the ordinary session path.

The event subscription, sender ID, event ID, root, action, readiness, and
outcome exist only inside the short-lived host diagnostic. No protocol field,
capability, SDK method, child input, callback, acknowledgement, or assistive-
technology-presence signal is added.

## Consequences

- A real Windows client verifies registration, delivery, and the existing
  authenticated status-replacement path without changing product behavior.
- The test does not prove Narrator speech, Inspect rendering, repeated events,
  silent refused states, or a delivery guarantee.
- One small direct COM event-handler adapter remains diagnostic-only and does
  not become a reusable application event API.

## Revisit conditions

Revisit before adding listener access, readiness signals, another event kind,
multiple status regions, hidden announcements, application-visible delivery,
or a non-Windows probe.
