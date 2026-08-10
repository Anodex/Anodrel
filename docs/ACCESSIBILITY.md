# Anodrel Windows accessibility

**Status:** Contract, mapping, and the **first** provider slice are implemented.
An Anodrel window now answers UI Automation as a server-side provider, and a
real UI Automation client reads its name, control type, and automation ID from
that provider.

Semantic children are **not** published yet, so a screen reader announces the
window and finds nothing inside it. Accessibility support is **not complete**,
and must not be described as complete until the Narrator and Inspect checks
below have actually been run by a person.

## Boundary

Anodrel already derives a bounded, source-ordered accessibility snapshot from a
validated `UiDocument` and one concrete `UiLayout` (Decision 0026). Each visible
node carries an element ID, a role, an optional plain-text name, clipped logical
bounds, and an enabled flag. That snapshot is portable data and performs no
operating-system call.

This document defines the layer directly above it: the Windows adapter that
turns one snapshot into the values Microsoft UI Automation asks for.

The boundary runs in one direction only:

~~~text
UiDocument + UiLayout          (portable, already validated)
        │
        │ accessibility_snapshot()
        ▼
UiAccessibilitySnapshot        (portable semantics, no OS call)
        │
        │ anodrel-windows-accessibility
        ▼
UIA control types, property values, runtime IDs, screen rectangles
        │
        │ (not implemented yet) UI Automation provider
        ▼
Windows, and any assistive technology it serves
~~~

**Nothing flows back.** An application cannot read the accessibility tree,
learn that assistive technology is present, discover which node is focused, be
notified that something was read aloud, or receive any event originating from a
screen reader. Whether a user relies on assistive technology is not observable
through this boundary, for the same reason a notification cannot report that it
was seen.

## What an application never touches

An application supplies a UI document and nothing else. It cannot:

- obtain or pass a window handle, provider pointer, or any native object;
- see UI Automation property IDs, control type IDs, runtime IDs, or patterns;
- register a provider, raise an automation event, or force focus;
- supply its own accessible role, override a mapping, or add a property; or
- learn that an assistive technology is running, connected, or reading.

Every value Windows receives is derived by the host from semantics the
application already declared. There is no accessibility-specific field in the
document format and none is planned: a role that cannot be expressed by the
existing model is a gap in the model, not something to bolt on here.

## Mapping

The adapter is a pure function from one snapshot node to Windows values.

| Anodrel role | UIA control type | Keyboard focusable |
| --- | --- | --- |
| `Group` | `Group` (50026) | no |
| `StaticText` | `Text` (50020) | no |
| `Button` | `Button` (50000) | yes |

| UIA property | Source |
| --- | --- |
| `Name` (30005) | The node's plain-text name, or empty where the role has none. |
| `ControlType` (30003) | The table above. |
| `IsEnabled` (30010) | The node's enabled flag. Only a button is ever disabled. |
| `AutomationId` (30011) | The document element ID. |
| `IsKeyboardFocusable` (30009) | The table above. |
| `IsControlElement` (30016) | Always true; every node in the snapshot is visible. |
| `IsContentElement` (30017) | Always true, for the same reason. |
| `BoundingRectangle` (30001) | Converted as below. |

`AutomationId` carries the element ID the application authored. It is a semantic
identifier already present in the document, it is bounded to 64 ASCII
characters, and assistive technology and UI test tooling both rely on a stable
one. It is not a path, handle, or secret.

Anything not in this table is deliberately absent. In particular there is no
`HelpText`, `AcceleratorKey`, `AccessKey`, `LocalizedControlType`, or pattern
provider: each would be a new promise to keep, and none has a source in the
current model.

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
origin.

### Runtime IDs

A runtime ID is `[UiaAppendRuntimeId, index]`, where the index is the node's
position in the source-ordered snapshot.

That is stable for as long as the tree is, which is exactly the required
lifetime: replacing the UI document produces a new tree, and its runtime IDs are
expected to differ. `UiaAppendRuntimeId` prefixes the host window's own ID, so
identifiers stay unique across windows without the adapter inventing a process
registry.

## Threading

UI Automation calls arrive on the UI thread through `WM_GETOBJECT`. The same
rule that keeps a pipe worker away from User32 and Shell32 applies: an
authenticated session worker never serves an accessibility request. The mapping
itself is pure and holds no lock, so it cannot block a message pump.

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

It is **read-only**: `GetPatternProvider` returns nothing for every pattern, so
no element can be invoked, toggled, scrolled, or edited through it. Each COM
method contains panics and converts one into a failure code, because these are
`extern "system"` and an escaping panic would abort the host.

The provider is built only when `UiaClientsAreListening` reports a client. That
answer never leaves the crate: exposing it would tell an application that
somebody is using assistive technology.

**Slice 2 — semantic children. Not implemented.**
`IRawElementProviderFragment` and `IRawElementProviderFragmentRoot`, tree
navigation over the mapped nodes, `GetRuntimeId` with its safe array, and
`get_BoundingRectangle`.

Until that exists a screen reader announces the window and finds **nothing
inside it**, because the mapped elements are not yet reachable from the root.

**Slice 3 — verification.** Narrator and Inspect, by a person. See below.

Also deferred, each needing its own contract and decision: automation events and
live announcements, focus changes reported to assistive technology, action
invocation through the `Invoke` pattern, text patterns and ranges, relations
between nodes, and non-Windows accessibility adapters.

## Verification

Automated tests cover the mapping: every role's control type and focusability,
each property's source, empty and named nodes, rectangle conversion at several
scales and origins, an empty rectangle staying empty, and runtime-ID shape and
uniqueness within a snapshot.

The mapping is pure, so those tests need no window and no assistive technology.

Provider tests cover the COM object without Windows: the interfaces it answers
and the one it refuses, a refused query clearing its output, every method
rejecting a null output rather than writing through it, reference counting
freeing the object exactly once, a panicking body returning a failure code
instead of unwinding, and the read-only promise that no pattern is supplied.

### Confirmed against real UI Automation

Slice 1 has been queried by a real UI Automation client — `UIAutomationClient`
driving `AutomationElement.FindFirst` against a running `--ui-lab` window:

~~~text
Name         = Anodrel UI Lab
ControlType  = ControlType.Window
AutomationId = anodrel.surface
IsEnabled    = True
~~~

`AutomationId` is the decisive value. Windows' default window provider leaves it
empty, so reading `anodrel.surface` proves the host's own provider was accepted,
`QueryInterface` succeeded, and `GetPropertyValue` was called and its `BSTR`
read back correctly.

That is evidence the COM plumbing is sound. It is **not** evidence that the
surface is usable, because there are still no children to read.

### Manual screen-reader verification

**Accessibility support is not complete until this has been run by a person and
passed.** No automated result substitutes for it: the question is whether a
screen reader announces something a person can act on, and only listening
answers that.

Running it today will show the window announced and nothing inside it, because
slice 2 is not built. Run it in full once children are published:

1. Open a native UI surface, for example
   `cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --ui-lab`.
2. Start **Narrator** with `Ctrl+Windows+Enter`. Narrator ships with Windows and
   needs no installation.
3. Move through the surface with `Caps Lock+Left/Right`. Each visible element
   should be announced with its name and its role — "button", "text", or
   "group".
4. Confirm a disabled action is announced as unavailable, and that an element
   clipped out of view is not announced at all.
5. Optionally cross-check with **Accessibility Insights for Windows** or the
   Windows SDK's **Inspect** tool, which show the raw UI Automation tree
   including `AutomationId`, `ControlType`, `IsEnabled`, and
   `BoundingRectangle`. Confirm each matches the mapping table above and that
   the highlighted rectangle sits over the element on screen.
6. Close Narrator with `Ctrl+Windows+Enter`.

Report a mismatch between what Inspect shows and what this document promises as
a defect in the adapter, not in the document: the table above is the contract.

See `docs/UI.md`, Decision 0026, and Decision 0063.
