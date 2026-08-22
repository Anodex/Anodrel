# Decision 0100: Live status is visible, semantic, and outbound-only

**Status:** Accepted

**Date:** 2026-08-21

## Context

The Windows UI Automation provider can already describe a current Anodrel
surface and notify Windows that a whole authenticated-session document was
replaced. That lets a screen reader refresh its tree, but it does not identify
the small, user-facing result that should be spoken after an operation
completes. Applications need an accessible status surface without gaining an
automation callback, listener signal, native handle, or a generic event API.

A document field that directly selects UI Automation properties would violate
Decision 0063: accessibility remains derived from the owned UI model. Treating
every changed text node as a live region would be worse: ordinary layout,
replacement, and field-preserving updates could become noisy speech.

## Decision

Add one semantic `Status` node to the portable UI model and exact
`anodrel.ui.document.v3` format. It is visible text with an explicit semantic
urgency of `polite` or `assertive`; it is not hidden accessibility metadata.
Every document may contain at most one status node. It uses the same bounded
single-line text, element-ID, font-size, layout, clipping, paint, and document
limits as a text node.

Version 3 retains every version 2 node unchanged and adds `status`. It is
accepted only through explicit Protocol 1.26 version-3 document operations;
v1 and v2 continue to reject it. The primary-session and session-window v3
operations reuse the existing `ui.document.write` grant because this changes
the declared visible UI, not operating-system authority. Existing v1 and v2
operations keep their exact contracts.

The portable accessibility snapshot maps a status to its own `Status` role;
the Windows adapter publishes it as UI Automation `Text` with
`LiveSetting=Polite` or `Assertive`. There is no caller-selected control type,
property identifier, provider, or event identifier.

After a native session view applies a strictly newer accepted v3 snapshot, the
host compares its one status node with the previous accepted one. It raises
`UIA_LiveRegionChangedEventId` only when all of these are true:

- the view had already applied an earlier authenticated document;
- the new status exists and its ID, value, or urgency differs from the prior
  status;
- the new status is visible in the current clipped layout; and
- the current immutable provider still contains that node as a live status.

The first document of a view merely establishes the baseline and never
announces. Removing a status, replacing a document with the same status,
resizing, scrolling, painting, typing, field-value change, focus, action,
dialog, notification, and a rejected or stale mailbox snapshot raise nothing.
A clipped update remains silent rather than making off-screen content audible.

The event source is a fresh immutable child provider. It occurs only after the
view-registry lock has been released. The HRESULT is discarded: Anodrel does
not call `UiaClientsAreListening`, retain subscribers, log delivery, rate a
screen reader, or expose a protocol result, callback, capability, or
accessibility-presence signal. Diagnostics, previews, and static package
surfaces may render a status but never raise an application status event.

## Consequences

- An application can make one visible operation result accessible through the
  same validated document it draws.
- Screen readers receive a standard one-way notification only for a meaningful
  later change, not initial static content or arbitrary text churn.
- The event has bounded immutable lifetime and creates no new cross-process
  control or observation channel.
- A surface needing multiple simultaneous regions, rich text, progress values,
  announcements without visible text, or a delivery guarantee needs another
  model and event decision.

## Revisit conditions

Revisit before adding multiple status regions, hidden announcements, progress
or alert controls, live-region removal events, rate or queue configuration,
property/value/text events, listener tracking, application-visible delivery,
or a non-Windows adapter.
