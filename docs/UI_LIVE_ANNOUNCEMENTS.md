# Anodrel UI live status announcements

**Status:** Implemented on Windows for authenticated version-3 UI sessions;
manual Narrator and Inspect verification is required.**

## Purpose

Anodrel can expose one visible, bounded status result to assistive technology
without giving an application any information about who heard it. This is not
a general message bus or a hidden screen-reader channel. It is one semantic
text node in the UI a person can see, carried by a complete validated document
replacement.

Decision 0100 defines the boundary. See `docs/UI_DOCUMENTS.md` for the exact
version-3 JSON and `docs/ACCESSIBILITY.md` for its Windows mapping.

## Status node

A version-3 document may contain **at most one** `status` node. Its value is
ordinary visible single-line text, and its `politeness` is exactly `polite` or
`assertive`.

- `polite` asks assistive technology to speak the update without interrupting
  its current speech.
- `assertive` marks an urgent visible result; applications should use it
  sparingly because the operating system may interrupt speech.

The status has an element ID, font size, and text tone like `text`. It is
painted, measured, wrapped, and clipped as ordinary text. It has no action,
focus, callback, timer, native operation, or protocol result. A clipped status
remains in the accessibility tree with `IsOffscreen=true`, but is never used
as a live-event source.

## Exact Windows behavior

The Windows UI Automation adapter reports a status as a `Text` control with
`LiveSetting` set to `Polite` (`1`) or `Assertive` (`2`). Other Anodrel nodes
report `Off` (`0`). On a qualifying change, the host raises one
`UIA_LiveRegionChangedEventId` (`20024`) from a fresh provider for the new
visible status node.

The host compares the old and new semantic status after it accepts a newer
document from the authenticated session mailbox. It raises at most one event
for that applied snapshot, and only when the new status's element ID, text, or
politeness differs. The first document in a view establishes a baseline and is
silent. Removing a status is silent because there is no present visible source
to announce.

An event is best effort. Its result is discarded, not logged, and cannot alter
the accepted document or local rendering. The host neither checks for a UI
Automation listener nor stores one, so an application cannot detect assistive
technology or learn whether the announcement was heard.

## Boundaries

- No new capability, installed-record field, or application permission is
  introduced. The existing `ui.document.write` grant remains necessary for the
  version-3 replacement operations.
- No application can pass a UI Automation property, event identifier, native
  object, provider pointer, callback, or recipient.
- No app reads a screen-reader state, announcement history, queue depth,
  delivery result, focus target, or automation tree.
- Host diagnostics, developer previews, static text packages, typing, focus,
  pointer input, scrolling, resizing, and native notifications do not emit
  live-region events.
- V1 and v2 documents reject `status`; a v3 document is never accepted by an
  older operation that might ignore its semantics.

## Verification

Automated tests cover exact v3 decoding and encoding, the one-status limit,
v1/v2 rejection, portable snapshot and Windows mapping values, provider
property publication, and every event gate: initial, unchanged, removed,
clipped, stale, and changed-visible statuses.

Manual Windows verification uses the built-in `--sample-ui-live-status-client`
route from `docs/DEVELOPMENT.md`. Open its window, then start Narrator, so the
check covers the order that originally found the listener-gate defect. Establish
the sample's visible initial status, activate **Publish visible result**, and
observe one distinct polite and one distinct assertive later value. Each must
be announced once and Inspect must show the matching `LiveSetting` on the
visible `Text` element. Replacing the document with the same status, removing
it, or changing an off-screen status must not announce. This manual check is
required before calling the feature manually verified.
