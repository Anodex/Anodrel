# Anodrel Windows UI Automation scroll contract

**Status:** Implemented and covered by focused UI Automation and Windows-host
tests. Manual Narrator and Inspect verification remains required.

## Purpose

The direct Windows host already retains a vertical position for each visible v2
scroll viewport. This contract makes exactly one of those positions operable by
Windows assistive technology without turning scrolling into application input or
application state.

It uses the Windows UI Automation `ScrollPattern` / `IScrollProvider` interface.
The platform supplies no browser, webview, toolkit, or third-party accessibility
runtime.

Decision 0098 adds the companion `ScrollItemPattern` for eligible descendants
of this same target. Its separate contract is
`docs/UI_AUTOMATION_SCROLL_ITEMS.md`.

## Published target

For one immutable accessibility publication, the only target is the first
visible overflowing scroll metric in source order. It must name the same
semantic `Scroll` group, viewport height, content height, and retained offset
that produced the publication's layout.

The target keeps its ordinary `Group` control type. The pattern does not alter
the portable semantic model or the document format.

No pattern is published for:

- the window root;
- a group that is not a scroll viewport;
- a viewport whose content currently fits;
- a later or nested scroll viewport;
- static text, an edit, or a button; or
- a provider without the host's scroll route.

## Values

The provider reports vertical-only scrolling.

| UI Automation value | Result |
| --- | --- |
| `HorizontallyScrollable` | false |
| `HorizontalScrollPercent` | `-1` (`NoScroll`) |
| `HorizontalViewSize` | 100 |
| `VerticallyScrollable` | true |
| `VerticalScrollPercent` | `offset / maximumOffset * 100`, clamped to 0–100 |
| `VerticalViewSize` | `viewportHeight / contentHeight * 100`, clamped to 0–100 |

All published values are finite copies. A fresh `WM_GETOBJECT` reply can reflect
a later local scroll position; a provider already held by a client cannot read
live host state.

## Operations

`Scroll(horizontal, vertical)` accepts only `NoAmount` horizontally. Vertically:

- `SmallDecrement` and `SmallIncrement` use the existing host-local line
  operation;
- `LargeDecrement` and `LargeIncrement` use the existing host-local page
  operation; and
- `NoAmount` leaves the position unchanged.

`SetScrollPercent(horizontal, vertical)` accepts only horizontal `-1` and a
finite vertical value from 0 through 100. The host maps that percentage into
its existing clamped absolute offset. A request at either limit is accepted even
when it is already at that limit; the absence of a visual change is not a
failure.

The provider rejects unsupported horizontal movement, invalid scroll amounts,
non-finite percentages, and values outside the standard range. It does not
expose a route to another viewport or a reason why a valid request was refused.

## Thread and authority boundary

`IScrollProvider` can be called off the window's UI thread. It therefore offers
one closed command to a one-request mailbox, waits for at most 250 ms, and
wakes the UI thread with a private message containing no payload. The command
contains only:

- the provider's optional authenticated document revision;
- the selected semantic scroll viewport ID; and
- line, page, percentage, or separately validated item-reveal movement.

The UI thread takes that request once, verifies its current document revision,
rebuilds its current layout, and confirms the same ID is still the first
overflowing metric. Only then does it change the corresponding host-retained
`UiScrollState`. A stale, timed-out, invalid, or superseded request changes
nothing.

Neither direction crosses the application boundary:

- an application cannot create a ScrollPattern, choose a target, style a
  scrollbar, receive a scroll event, read a position, or learn that assistive
  technology is connected;
- an automation client receives no native handle, document, pointer stream,
  field value, application callback, or reason a request failed; and
- scrolling does not create a protocol operation, grant, event, revision,
  focus change, field change, or semantic action.

## Explicitly deferred

Automation property-change events, horizontal movement, nested target
arbitration, touch and kinetic movement, application-owned scroll state,
persistence, and non-Windows adapters are outside this slice. Scroll-item
reveal is specified separately by Decision 0098 so it cannot accidentally
widen the direct ScrollPattern contract.

## Verification plan

Automated coverage must prove:

- finite snapshot percent and view-size calculations;
- the pattern and interface appear only on the selected overflowing group;
- all unsupported directions and malformed values fail closed;
- line, page, and percent commands preserve revision and target binding;
- one busy or timed-out route cannot apply later; and
- the UI thread uses the same retained scroll state as pointer, keyboard, and
  wheel input.

After automated checks pass, a Windows manual check will use Narrator and
Inspect or Accessibility Insights to confirm the group advertises
`ScrollPattern`, its vertical values match the visible thumb, `PageDown` moves
the viewport, and an older provider does not mutate with later scrolling.
