# Anodrel Windows accessibility

**Status:** **UI Automation reading, host-owned vertical scrolling, bounded
scroll-item reveal, and live-status events are implemented. A direct,
first-party property/tree/geometry probe now verifies the current fixed
hierarchy in raw and control views, plus one fixed field rectangle and hit
target, against real Windows UI Automation; Narrator and Inspect verified the
earlier flat semantic surface. Manual hierarchy, scrolling and item reveal,
button invocation, focus, focus-event, field-value, structure-event, and
live-status screen-reader checks remain open.**
Narrator reads an Anodrel surface aloud on Windows 11, announcing each element
with its name and role, and a property-by-property cross-check against the
pre-hierarchy mapping table passed with no failures.

Reading, one bounded action, focus reporting and control, one host-raised
focus-change event, one host-raised document-replacement structure event, one
host-raised live-status event, read-only field values, one host-owned vertical ScrollPattern, and its bounded
ScrollItem companions are the implemented surface.
Assistive technology can read this surface, obtain a visible field's current
value, invoke an enabled button in an authenticated UI session, move to a
visible enabled field or button through the host's existing focus state, and
ask the selected host-owned scroll viewport to reveal one of its bounded
off-screen descendants. An
application cannot edit a field through automation, raise or receive an
automation event, learn whether a live announcement was delivered, or observe focus. The published
tree preserves the document's direct semantic parent/child structure. Anything
beyond these routes is deferred and listed at the end of this document.

## Boundary

Anodrel already derives a bounded, source-ordered accessibility snapshot from a
validated `UiDocument` and one concrete `UiLayout` (Decision 0026). Each
bounded node carries an element ID, a role, an optional plain-text name, clipped
logical bounds, an enabled flag, and its direct semantic parent's earlier
source-order index when it has one. A node wholly clipped out of view remains in
the tree with empty bounds and `IsOffscreen=true`; it is not locally
interactive. That snapshot is portable data and performs no operating-system
call.

This document defines the layer directly above it: the Windows adapter that
turns one snapshot into the values Microsoft UI Automation asks for.

The boundary runs in one direction only:

~~~text
UiDocument + UiLayout                          UiFieldStates
        │                                  (host-owned current text)
        │ accessibility_snapshot()                    │
        ▼                                             │ copied only for
UiAccessibilitySnapshot                              │ matching visible Edit
        │                                             │
        │ anodrel-windows-accessibility               │
        ▼                                             │
UIA control types, properties, runtime IDs, rectangles │
        └──────────────────────────┬──────────────────┘
                                   ▼
anodrel-windows-uia (provider, bounded Invoke, read-only Value, ScrollItem)
                                   │
                                   ▼
Windows, and any assistive technology it serves
~~~

The semantics themselves flow one way: an application cannot read the
accessibility tree, learn that assistive technology is present, discover which
node is focused, or be notified that something was read aloud. Whether a user
relies on assistive technology is not observable through this boundary, for the
same reason a notification cannot report that it was seen. The one exception is
an enabled button's ordinary semantic action: it travels through the same
revision-bound `ui.events.read` mailbox as a local click, not through an
accessibility-specific callback or observation channel. Decision 0069 defines
that narrow route.

## What an application never touches

An application supplies a UI document and nothing else. It cannot:

- obtain or pass a window handle, provider pointer, or any native object;
- see UI Automation property IDs, control type IDs, runtime IDs, or patterns;
- register a provider, raise an automation event, or force focus;
- supply its own accessible role, override a mapping, or add a property; or
- learn that an assistive technology is running, connected, or reading.

Every structural value Windows receives is derived by the host from semantics
the application already declared. A field's text is the narrow exception: it is
copied from the host-owned current field state only into a matching visible
`Edit` provider, never from an application request or accessibility-specific
document field. There is no application-supplied accessibility data in the
document format: a role that cannot be expressed by the existing model is a gap
in the model, not something to bolt on here.

## Mapping

The adapter is a pure function from one snapshot node to Windows values.

| Anodrel role | UIA control type | Keyboard focusable |
| --- | --- | --- |
| `Group` | `Group` (50026) | no |
| `StaticText` | `Text` (50020) | no |
| `Status` | `Text` (50020) | no |
| `Edit` | `Edit` (50004) | yes |
| `Button` | `Button` (50000) | yes |

An `Edit` reports as keyboard focusable because Tab really does reach it. This
table matches the portable focus traversal exactly, and reporting a field as
unreachable would be a plain lie to a screen reader; the outbound semantics
rule below is about keeping the mapping truthful without creating an
accessibility-specific callback or observation channel.

An `Edit` is named by its label and additionally exposes its current value
through the read-only Value pattern defined below. The value is copied from the
host's field state for this provider snapshot; it is not the document's initial
value, an application read, or a typing event. See `docs/UI_FIELDS.md` and
Decision 0071.

### Tree structure

The snapshot is a preorder walk: a node's optional parent index always names
its direct, earlier semantic ancestor. The Windows adapter preserves every
mapped node, including `Group` and a fully clipped node with empty bounds, and
the provider derives immutable direct parent and child lists from those indices.
`Parent`, `FirstChild`, `LastChild`, `NextSibling`, and `PreviousSibling`
therefore describe the declared bounded document hierarchy; top-level snapshot
nodes belong to the host-owned window root.

Groups are structural only. They are not keyboard focusable and expose no
action or value pattern. A currently selected overflowing `Scroll` group is the
one exception: Decision 0097 gives a selected overflowing `Scroll` group the
host-owned vertical `ScrollPattern`, while Decision 0098 gives eligible
descendants of that same group `ScrollItemPattern`. Both are described in
`docs/UI_AUTOMATION_SCROLL.md` and `docs/UI_AUTOMATION_SCROLL_ITEMS.md`, without
changing a Group's control type or document semantics. An unnamed group remains unnamed because the
portable document model has no group-label field. Hit testing returns the
deepest mapped element containing a point; overlapping siblings follow source
paint order, with the later sibling winning.

The structure is an immutable provider snapshot. It adds no arbitrary relation,
live view lookup, structure event, application callback, document field,
protocol operation, capability, or way for an application to inspect assistive
technology. A fresh UI Automation query can see a replacement document's new
snapshot; an older provider never changes underneath a client. Decision 0075
defines this boundary.

| UIA property | Source |
| --- | --- |
| `Name` (30005) | The node's plain-text name, or empty where the role has none. |
| `ControlType` (30003) | The table above. |
| `IsEnabled` (30010) | The node's enabled flag **for a button or a field**; always true for text and groups. |
| `AutomationId` (30011) | The document element ID. |
| `IsOffscreen` (30022) | True exactly when the node's clipped bounds are empty. |
| `IsKeyboardFocusable` (30009) | The table above. |
| `HasKeyboardFocus` (30008) | The host-owned focus snapshot, true only for its current published focus target. |
| `Value.Value` (30045) | The copied current host field text, only on a matching visible `Edit`. |
| `Value.IsReadOnly` (30046) | Always true on that `Edit`, because UI Automation has no write route. |
| `IsControlElement` (30016) | Always true for a bounded semantic node, whether currently clipped or visible. |
| `IsContentElement` (30017) | Always true, for the same reason. |
| `BoundingRectangle` (30001) | Converted as below. |
| `LiveSetting` (30135) | `Off` except a semantic `Status`, which maps to its declared `Polite` or `Assertive` setting. |

`IsEnabled` deserves its exception. UI Automation reads it as "can be interacted
with", and a screen reader announces a disabled element as unavailable. Only an
action or field can be unavailable; text and containers are not interactive in
the first place, so passing the snapshot's flag straight through would have Narrator
describe ordinary prose as dimmed and out of reach. Real UI Automation
verification is what caught this.

`AutomationId` carries the element ID the application authored. It is a semantic
identifier already present in the document, it is bounded to 64 ASCII
characters, and assistive technology and UI test tooling both rely on a stable
one. It is not a path, handle, or secret.

Anything not in this table is deliberately absent. In particular there is no
`HelpText`, `AcceleratorKey`, `AccessKey`, or `LocalizedControlType`. The
implemented bounded patterns are described below; Decisions 0097 and 0098
implement a vertical `ScrollPattern` for the one host-selected overflowing
scroll group and `ScrollItemPattern` for its eligible descendants. Neither adds
an application accessibility field or pattern choice.

### Button invocation

An enabled visible `Button` in an authenticated UI session exposes `Invoke`
(10000) through `IInvokeProvider`. No other role, disabled or clipped button,
window root, or diagnostic surface exposes it.

`Invoke` offers exactly one `ActionInvoked(element_id)` candidate, bound to the
revision whose layout produced the provider, to the session's existing bounded
input mailbox. It neither synthesizes a native click nor calls application code.
The existing `ui.events.read` validation decides whether an application may
receive it, so a provider held across document replacement cannot activate a
stale, removed, or disabled action. A full mailbox returns a generic failure and
does not create another queue or disclose capacity through UI Automation.
This adds no protocol field, grant, operation, or version: it is one more
host-owned producer for the existing revalidated semantic-action route.

The UI Lab is a host diagnostic: its action tiles are local and it has no
application-session mailbox. It therefore remains readable but has no Invoke
pattern. This is intentional; a diagnostic must not quietly become a second
application action route.

### Focus reporting

The provider receives the existing host-owned `UiFocus` alongside the document
layout it publishes. `GetFocus` returns that one matching child and
`HasKeyboardFocus` is true only on that child. A missing, clipped, disabled,
non-focusable, or non-published ID produces no focused element instead of a
guess.

This is an immutable provider snapshot. A new UI Automation query observes a
later keyboard or pointer focus change; an older provider does not read the
live view or registry to chase it. `SetFocus` is the one deliberate action:
only a visible enabled focusable child can hand one revision-bound request to
its owning UI thread, which revalidates that same target before it changes the
host focus state. Windows focuses the containing fragment before calling it, so
Anodrel does not activate a window or send input. After a genuine focus move,
the host raises one standard focus-change event from a fresh immutable provider
for the new target. It has no application callback, protocol field, capability
grant, listener check, or result surface. Decisions 0070, 0073, and 0074
define these boundaries; `docs/UI_AUTOMATION_FOCUS.md` and
`docs/UI_AUTOMATION_EVENTS.md` give the exact routes.

### Field value reading

A visible `Edit` whose current host field state has the same element ID exposes
`Value` (10002) through `IValueProvider`. Its `Value` is a copied immutable
UTF-16 snapshot. A fresh UI Automation query can observe a later local edit;
an older provider never reads the live registry to chase it.

`Value.IsReadOnly` is true and `SetValue` returns `UIA_E_NOTSUPPORTED` for
every field, including an enabled field. Read-only here describes automation
authority, not whether a person can use the field: the host continues to accept
only local keyboard and pointer input. No selector, caret, selection, text
range, value-change event, native input message, application callback, protocol
field, grant, or version is added. A disabled field can still report its visible
value; `IsEnabled` independently reports that a person cannot edit it.

The BSTR returned to a UI Automation client is its own COM allocation. A client
can retain that copy, so this feature is for ordinary visible v1 text only;
Anodrel still has no password or masked field. Decision 0071 defines the
boundary.

### Scroll-item reveal

The first visible overflowing `Scroll` group exposes its existing vertical
`ScrollPattern`. Each bounded descendant whose nearest semantic scroll ancestor
is that group exposes `ScrollItemPattern` (10017) through
`IScrollItemProvider`, including a descendant whose clipped rectangle is empty.
The scroll group itself does not expose ScrollItem, and a descendant inside a
nested scroll group is not an item of the outer group. A nested scroll group may
be revealed as an outer item; routing into its contents remains deferred.

`ScrollIntoView` has no target, offset, alignment, focus, or callback input.
It takes the item's already-published semantic ID and the selected viewport ID
through the same 250 ms revision-bound private route as `IScrollProvider`. The
owner UI thread checks the current revision, selected first visible overflowing
viewport, nearest scroll ancestor, and current layout before it changes the
existing retained offset. It aligns a smaller item to its nearest viewport edge,
an oversized item to the top, and accepts an already visible item without a
change. A fresh provider can then show the new visibility; the old one remains
immutable.

This is a visibility route only. It neither moves focus nor exposes a field
value or Invoke pattern while the target remains clipped. It emits no automation
event and changes no application document, protocol state, semantic action, or
capability. `docs/UI_AUTOMATION_SCROLL_ITEMS.md` and Decision 0098 define the
full contract.

### Bounding rectangles

The snapshot holds clipped **logical** pixels relative to the client area. UI
Automation wants **physical** pixels in screen space, as `[left, top, width,
height]` doubles.

Conversion takes the client area's screen origin and the window's current scale
from the host, both supplied by the caller rather than queried inside the
mapping. That keeps the conversion a pure function and lets it be tested at
every supported display density without a window.

An empty rectangle stays empty rather than becoming a degenerate point, so a
node clipped entirely out of view cannot be reported as a target at the client
origin. That same node reports `IsOffscreen=true`; a partially clipped node
retains its visible clipped rectangle and remains `IsOffscreen=false`.

### Runtime IDs

A runtime ID is `[UiaAppendRuntimeId, index]`, where the index is the node's
position in the source-ordered snapshot.

That is stable for as long as the tree is, which is exactly the required
lifetime: replacing the UI document produces a new tree, and its runtime IDs are
expected to differ. `UiaAppendRuntimeId` prefixes the host window's own ID, so
identifiers stay unique across windows without the adapter inventing a process
registry.

## Threading

`WM_GETOBJECT` arrives on the UI thread, which creates the immutable provider
snapshot. A later UI Automation method can arrive from an automation caller, so
the mutating methods, `SetFocus`, `IScrollProvider`, and
`IScrollItemProvider`, use bounded private request routes back to that owner. A
pipe worker never serves an accessibility request, and an automation caller
never receives a mutable view or registry entry. The mapping itself is pure and
holds no lock, so it cannot block a message pump.

## Failure behaviour

The mapping cannot fail. Every snapshot node produces a complete set of values,
and a node with no name produces an empty name rather than an error. There is no
category to report, and therefore nothing an application could learn from one.

## The provider, in slices

The provider is built in stages so each one can be proved before the next
begins.

**Slice 1 — the window as an automation root. Implemented.**
`anodrel-windows-uia` answers `WM_GETOBJECT` for `UiaRootObjectId` with a
reference-counted `IRawElementProviderSimple`. It reports the window's name,
`Window` control type, a fixed host-owned automation ID, and its enabled and
element flags, and defers everything else to the host provider Windows supplies.

The root is **read-only**: `GetPatternProvider` returns nothing for every
pattern, so the window itself cannot be invoked, toggled, scrolled, or edited.
Each COM method contains panics and converts one into a failure code, because
these are `extern "system"` and an escaping panic would abort the host.

The provider is returned **whenever the root object is asked for**, without
first checking whether a client is listening. `UiaClientsAreListening` answers
whether raising an *event* is worthwhile, not whether to supply a provider;
using it as a gate meant a window created before a screen reader started
answered its early requests with nothing and was resolved to the default window
provider instead.

**Slice 2 — semantic children. Implemented.**
The window is also an `IRawElementProviderFragmentRoot`, and each bounded
published element answers `IRawElementProviderFragment`: navigation,
`GetRuntimeId` as a safe array, `get_BoundingRectangle`, `IsOffscreen`, and hit
testing from a screen point.

`SetFocus` succeeds only for a published visible enabled focus target. It uses
the host-owned route described in `docs/UI_AUTOMATION_FOCUS.md`; the root,
static text, disabled elements, clipped elements, stale session providers, and
unavailable views fail. `GetFocus` remains a snapshot lookup rather than a live
view query.

The published tree preserves direct semantic parentage, including `Group`
containers and fully clipped descendants. Fragment navigation reports direct
parents, children, and siblings from that immutable snapshot; it never consults
a mutable view or registry. Groups remain non-focusable and pattern-free unless
they are the selected ScrollPattern group or an eligible ScrollItem child. The
existing reading checks prove the former flat surface; the manual hierarchy
check below remains required for this newly structural tree.

**Slice 3 — reading verification. Partly complete.** Narrator announced the
earlier flat surface's elements with their names and roles, and each then-
published property was cross-checked against the table. The new hierarchy has
its own manual check below and is not inferred from that result.

**Slice 4 — bounded button invocation. Implemented; manual activation check
pending.** An enabled visible authenticated-session button exposes
`IInvokeProvider`. Its `Invoke` implementation writes only a revision-bound semantic candidate into
the same bounded mailbox as native pointer and keyboard activation (Decision
0069). It does not move focus, synthesize a Windows input message, or call an
application. A full mailbox fails without adding a queue. The unit and host
tests prove the role/enabled/session gates and the exact candidate route; the
manual check below must still prove Narrator can activate a button.

**Slice 5 — focus reporting. Implemented; manual focus check pending.** The
provider returns the matching child from `GetFocus` and sets
`HasKeyboardFocus` only on that element, using one immutable snapshot of the
host's existing layout-validated focus (Decision 0070).

**Slice 6 — read-only field values. Implemented; manual value check pending.**
A matching visible `Edit` exposes `IValueProvider`, returns its copied host
value, and is read-only to automation (Decision 0071). It has no `SetValue`,
caret, selection, text range, or value-change event.

**Slice 7 — bounded focus control. Implemented; manual focus-control check
pending.** A visible enabled field or button can request focus through
`IRawElementProviderFragment::SetFocus`; a private per-window route returns to
the UI thread, which revalidates the provider revision and target before it
updates host focus (Decision 0073). It does not expose focus to an application,
activate a window, or send input.

**Slice 8 — host-only focus-change event. Implemented; manual event check
pending.** After a local pointer/Tab move or a changed accepted `SetFocus`, the
host raises `UIA_AutomationFocusChangedEventId` for a fresh immutable provider
of the new target (Decision 0074). A no-op, refusal, stale request, and any
other event kind remain absent.

**Slice 9 — host-only structure-change event. Implemented; manual event check
pending.** After the UI thread accepts and applies a strictly newer
authenticated session document, the window root raises one
`ChildrenInvalidated` event from a fresh provider (Decision 0076). It does not
name a listener, retain a subscription, or expose an application callback.
Stale or absent documents, layout, resize, typing, field changes, focus,
actions, dialogs, notifications, and closure raise nothing. See
`docs/UI_AUTOMATION_STRUCTURE_EVENTS.md`.

**Slice 10 — host-owned vertical scrolling. Implemented; manual scrolling
check pending.** The first visible overflowing `Scroll` group exposes
`IScrollProvider`. Its immutable provider snapshot reports vertical percentage
and view size only; small and large increments return through a 250 ms
revision-bound route to the owning UI thread, which confirms the same group is
still the first overflowing viewport before it changes the established retained
offset. The unit and host checks prove pattern gating, standard values, command
validation, mailbox timeout safety, revision/target revalidation, and reuse of
the pointer/wheel/keyboard scrollbar state. It has no event, application
callback, position readback, or horizontal/nested target. See
`docs/UI_AUTOMATION_SCROLL.md`.

**Slice 11 — host-owned scroll-item reveal. Implemented; manual scrolling and
item-reveal check pending.** Every bounded descendant whose nearest scroll
ancestor is the selected first visible overflowing group exposes
`IScrollItemProvider`; an off-screen element therefore remains navigable with
an empty rectangle and `IsOffscreen=true`. `ScrollIntoView` returns through the
same 250 ms revision-bound route and the owner revalidates viewport, target, and
current layout before it adjusts the retained offset. It has no alignment,
focus, action, value, event, application callback, position readback, or nested
route. The unit and host checks prove off-screen tree retention, interface and
pattern gates, nested refusal, nearest-edge geometry, timeout safety, and reuse
of the existing scroll state. See `docs/UI_AUTOMATION_SCROLL_ITEMS.md`.

**Slice 12 — semantic live status. Implemented; manual announcement check
pending.** A visible changed `Status` in an established authenticated v3
session document maps to UI Automation `Text` plus its `LiveSetting`, then
raises one `UIA_LiveRegionChangedEventId` from a fresh provider. Initial,
unchanged, removed, clipped, stale, diagnostic, and non-session values are
silent. There is no listener check, callback, result, or application event.
See `docs/UI_LIVE_ANNOUNCEMENTS.md` and Decision 0100.

Also deferred, each needing its own contract and decision: property/value/text/selection events beyond
the implemented live-status event, selection
and caret reporting, text patterns and ranges, labelled-by or described-by
relations, automation editing, horizontal or nested scroll automation, scroll
events, and non-Windows accessibility adapters.


## Verification

The repeatable and hands-on checks for this surface are maintained in [Accessibility verification](ACCESSIBILITY_VERIFICATION.md).
