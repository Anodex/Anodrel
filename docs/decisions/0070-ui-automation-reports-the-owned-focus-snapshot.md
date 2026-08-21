# Decision 0070: UI Automation reports the owned focus snapshot without controlling it

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0027 gives each native UI view a small host-owned `UiFocus` value.
Keyboard and pointer input already update it only through the current layout,
so focus cannot remain on an invisible or disabled field or button. The Windows
UI Automation provider published those focusable elements but answered
`GetFocus` with nothing and reported no focused element. A screen reader could
therefore find a control but could not learn which one the host keyboard was on.

There are two tempting but wrong ways to close that gap. Reading system focus
or retaining a mutable window view inside a provider would couple COM calls to
global window state. Implementing `SetFocus` or focus-change events at the same
time would make UI Automation a new control path before the reporting contract
is proved.

## Decision

Every `WM_GETOBJECT` provider tree receives one immutable focus snapshot from
the host's existing `UiFocus`, built with the same document revision and layout
as the elements it publishes. A focus ID is reported only when it names a
visible, enabled, keyboard-focusable published element in that exact tree.
Otherwise the tree has no focused element.

The provider uses that snapshot in exactly two places:

- `IRawElementProviderFragmentRoot::GetFocus` returns the matching child
  provider, or null when there is none; and
- `UIA_HasKeyboardFocusPropertyId` is true only on that matching child and
  false everywhere else, including the root.

The snapshot never reaches an application. No protocol field, grant, operation,
or version changes. The provider does not inspect current system focus, retain a
registry lock or mutable view, report focus change events, discover whether a
client is listening, or implement `SetFocus`. A provider held after focus moves
continues to describe the immutable tree it was created for; a fresh UIA query
observes the next host snapshot.

## Consequences

Positive:

- assistive technology receives the same current element identity that the
  keyboard focus model uses, without an additional focus model or native
  authority;
- a stale provider cannot read or mutate live window state; and
- focus behaviour remains testable as pure tree selection plus the existing
  host focus traversal tests.

Tradeoffs:

- a client does not receive a focus-change event and must ask Windows again to
  obtain a new snapshot;
- UI Automation still cannot set focus, so it cannot repair focus or drive an
  Anodrel surface; and
- manual Narrator and Inspect checks remain necessary to prove what a person
  hears and sees, beyond the provider and host tests.

## Revisit conditions

Revisit before adding `SetFocus`, a UI Automation focus-changed event, native
window activation, focus notifications to applications, text/value patterns, or
any way for an application to observe assistive technology. Each would change a
different authority or observation boundary.
