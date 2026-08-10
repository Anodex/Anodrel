# Anodrel Windows accessibility

**Status:** Contract and the portable-to-Windows mapping are defined. The UI
Automation provider that publishes that mapping to Windows is not implemented
yet, so no assistive technology can read an Anodrel surface today.

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

## Not implemented yet

The UI Automation **provider** is the remaining work: COM interfaces
(`IRawElementProviderSimple`, `IRawElementProviderFragment`,
`IRawElementProviderFragmentRoot`), `WM_GETOBJECT` handling through
`UiaReturnRawElementProvider`, and tree navigation over the mapped nodes.

Until that exists, an Anodrel window exposes only the default accessibility
Windows gives any top-level window: a title and a client area, with no semantic
children. **A screen reader cannot read an Anodrel UI surface today.**

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

### Manual screen-reader verification

This path cannot be exercised until the provider exists. When it does, the check
is:

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
