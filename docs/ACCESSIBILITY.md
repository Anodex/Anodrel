# Anodrel Windows accessibility

**Status:** **Read-only UI Automation support is implemented and verified.**
Narrator reads an Anodrel surface aloud on Windows 11, announcing each element
with its name and role, and a property-by-property cross-check against the
mapping table below passes with no failures.

Read-only is the whole of it. Assistive technology can read this surface; it
cannot act on it. No pattern is supplied, focus cannot be moved, no automation
event or live announcement is raised, and the published tree is flat. Anything
beyond reading is deferred and listed at the end of this document.

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
| `Edit` | `Edit` (50004) | no |
| `Button` | `Button` (50000) | yes |

An `Edit` is named by its **label**, never by its value. A field's text leaves
the host only through the granted snapshot of Decision 0067, and this tree is a
published surface — so assistive technology can find a field and cannot read
what is in it, or move focus into it. That is the existing one-directional rule
applied to the node where reading the value would matter most, and it is a real
limitation for a screen-reader user filling a form. Lifting it needs its own
decision rather than a quiet loosening of this one. See `docs/UI_FIELDS.md`.

| UIA property | Source |
| --- | --- |
| `Name` (30005) | The node's plain-text name, or empty where the role has none. |
| `ControlType` (30003) | The table above. |
| `IsEnabled` (30010) | The node's enabled flag **for a button or a field**; always true for text and groups. |
| `AutomationId` (30011) | The document element ID. |
| `IsKeyboardFocusable` (30009) | The table above. |
| `IsControlElement` (30016) | Always true; every node in the snapshot is visible. |
| `IsContentElement` (30017) | Always true, for the same reason. |
| `BoundingRectangle` (30001) | Converted as below. |

`IsEnabled` deserves its exception. UI Automation reads it as "can be interacted
with", and a screen reader announces a disabled element as unavailable. Only an
action can be unavailable; text and containers are not interactive in the first
place, so passing the snapshot's flag straight through would have Narrator
describe ordinary prose as dimmed and out of reach. Real UI Automation
verification is what caught this.

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

The provider is returned **whenever the root object is asked for**, without
first checking whether a client is listening. `UiaClientsAreListening` answers
whether raising an *event* is worthwhile, not whether to supply a provider;
using it as a gate meant a window created before a screen reader started
answered its early requests with nothing and was resolved to the default window
provider instead.

**Slice 2 — semantic children. Implemented.**
The window is also an `IRawElementProviderFragmentRoot`, and each published
element answers `IRawElementProviderFragment`: navigation, `GetRuntimeId` as a
safe array, `get_BoundingRectangle`, and hit testing from a screen point.

`SetFocus` returns `UIA_E_NOTSUPPORTED`. Moving focus is an action, and this
provider performs none. `GetFocus` returns nothing, because reporting focus to
assistive technology is its own slice and guessing would be worse than silence.

The published tree is **flat**, and groups are filtered out of it. A container
whose children sit beside it rather than inside it would be announced as an
empty thing to step through, which is worse than not publishing it. Hierarchy,
and with it meaningful grouping, is deferred.

**Slice 3 — verification. Done.** Narrator announces the surface's elements with
their names and roles, and every published property has been cross-checked
against the table above. See below.

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

Both slices have been queried by a real UI Automation client —
`UIAutomationClient` driving `FindFirst` and `FindAll` against a running
`--ui-lab` window. The window reports `AutomationId = anodrel.surface`, which
Windows' default window provider leaves empty, so the host's own provider was
accepted, `QueryInterface` succeeded, and `GetPropertyValue` returned a
correctly read `BSTR`.

Walking its children returns all eleven published elements with their control
types, names, automation IDs, enabled and focusable state, and screen
rectangles:

~~~text
[ControlType.Text]   enabled=True focusable=False name='NATIVE UI FOUNDATION'
[ControlType.Button] enabled=True focusable=True  name='Inspect layout'
~~~

That exercises navigation, property lookup, runtime identifiers, bounding
rectangles, and reference counting against real Windows.

**It is evidence the plumbing is sound, not that the surface is usable.** What a
client library reads and what a screen reader announces are different questions,
and only the second matters to a person. That is why the manual check below is
the gate.

This check also earned its keep: it caught text elements reporting
`IsEnabled=False`, which a screen reader announces as unavailable. No unit test
had flagged it, because the mapping faithfully passed through a flag that means
something different on each side of the boundary.

### Manual screen-reader verification

**No automated result substitutes for this.** The question is whether a screen
reader announces something a person can act on, and only listening answers it.

This has been run and **passed** on Windows 11: Narrator announced each element
with its name and role. It also earned its keep — see the note after step 7.

To repeat it:

1. Open a native UI surface, for example
   `cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --ui-lab`.
2. Start **Narrator**. `Ctrl+Windows+Enter` toggles it, but that shortcut can be
   disabled, so confirm it actually started rather than assuming: Narrator
   announces itself, and `Get-Process Narrator` lists it. If the shortcut does
   nothing, launch `%WINDIR%\System32\Narrator.exe` directly or enable it under
   Settings → Accessibility → Narrator.
3. **Click the Anodrel window** so it is in the foreground. Narrator reads the
   focused window, and item navigation does nothing while another window has
   focus.
4. Move through the surface with `Caps Lock+Left/Right`. Each visible element
   should be announced with its name and its role — "button" or "text".
5. Confirm a disabled action is announced as unavailable, and that an element
   clipped out of view is not announced at all.
6. Optionally cross-check with **Accessibility Insights for Windows** or the
   Windows SDK's **Inspect** tool, which show the raw UI Automation tree
   including `AutomationId`, `ControlType`, `IsEnabled`, and
   `BoundingRectangle`. Confirm each matches the mapping table above and that
   the highlighted rectangle sits over the element on screen.
7. Close Narrator with `Ctrl+Windows+Enter`.

If nothing is announced, check in this order: that Narrator is actually running,
that the Anodrel window has focus, and only then suspect the provider. A quick
way to separate a provider fault from a Narrator one is to walk the control view
with `TreeWalker::ControlViewWalker` from a UI Automation client — that is the
same view Narrator navigates, and it needs no screen reader.

### Inspect cross-check

Run with the Windows SDK's **Inspect** tool open on a `--ui-lab` window, every
published element was checked against the mapping table. All eleven passed with
no failures:

- control types are only `Text` and `Button`, matching their roles;
- every element has a non-empty name and automation ID;
- every element reports enabled, and only buttons report keyboard-focusable;
- every element is both a control and a content element;
- every bounding rectangle is non-empty and lies inside the window's own
  rectangle, and `AutomationElement.FromPoint` at each element's centre returns
  that same element — eleven of eleven, which exercises hit testing;
- runtime identifiers are unique across the tree;
- `HelpText` and `AcceleratorKey` are absent, as the table promises; and
- **no element supplies any pattern**, which is what read-only means at the
  client boundary.

The window root reports two patterns, `Window` and `Transform`. Those come from
the host provider Windows supplies for the `HWND` itself — minimising, moving,
and resizing a top-level window are the system's business, not this provider's.
Anodrel's own provider supplies no pattern anywhere, root included.

One incidental confirmation: the window under test sat on a monitor left of the
primary one, so its rectangle had negative screen coordinates throughout. The
conversion handles that, as its unit test claims.

### What these checks caught

The first attempt was silent, and the cause was real: provider creation was
gated on `UiaClientsAreListening`, so a window opened *before* Narrator started
answered its early requests with nothing. Every automated check had passed,
because they all attached a client to an already-live UI Automation session —
never the order a person actually works in.

Read alongside the `IsEnabled` defect that the UI Automation client query found,
the pattern is worth keeping: each layer of verification caught a fault the one
below it could not see. Unit tests proved the mapping, a client proved the COM
plumbing, and only a screen reader proved the sequence.

Report a mismatch between what Inspect shows and what this document promises as
a defect in the adapter, not in the document: the table above is the contract.

See `docs/UI.md`, Decision 0026, and Decision 0063.
