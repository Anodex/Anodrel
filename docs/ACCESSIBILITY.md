# Anodrel Windows accessibility

**Status:** **UI Automation reading is implemented and verified; bounded button
invocation, focus reporting, and read-only field values are implemented and
awaiting manual screen-reader checks.**
Narrator reads an Anodrel surface aloud on Windows 11, announcing each element
with its name and role, and a property-by-property cross-check against the
mapping table below passes with no failures.

Reading, one bounded action, focus reporting, and read-only field values are the
whole of it. Assistive technology can read this surface, obtain a visible
field's current value, and invoke an enabled button in an authenticated UI
session; it reports the host's current keyboard focus but cannot move it, edit a
field, raise an automation event, or receive a live announcement. The published
tree is flat. Anything beyond reading, button invocation, focus reporting, and
field-value reading is deferred and listed at the end of this document.

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
anodrel-windows-uia (provider, bounded Invoke, read-only Value)
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

| UIA property | Source |
| --- | --- |
| `Name` (30005) | The node's plain-text name, or empty where the role has none. |
| `ControlType` (30003) | The table above. |
| `IsEnabled` (30010) | The node's enabled flag **for a button or a field**; always true for text and groups. |
| `AutomationId` (30011) | The document element ID. |
| `IsKeyboardFocusable` (30009) | The table above. |
| `HasKeyboardFocus` (30008) | The host-owned focus snapshot, true only for its current published focus target. |
| `Value.Value` (30045) | The copied current host field text, only on a matching visible `Edit`. |
| `Value.IsReadOnly` (30046) | Always true on that `Edit`, because UI Automation has no write route. |
| `IsControlElement` (30016) | Always true; every node in the snapshot is visible. |
| `IsContentElement` (30017) | Always true, for the same reason. |
| `BoundingRectangle` (30001) | Converted as below. |

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
`HelpText`, `AcceleratorKey`, `AccessKey`, `LocalizedControlType`, or pattern
provider except the two bounded patterns below: each would be a new promise to
keep, and none has a source in the current model.

### Button invocation

An enabled `Button` in an authenticated UI session exposes `Invoke` (10000) through
`IInvokeProvider`. No other role, disabled button, window root, or diagnostic
surface exposes it.

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
live view or registry to chase it. There is no `SetFocus`, focus-change event,
window activation, application callback, protocol field, or capability grant.
Decision 0070 defines this boundary.

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
The window is also an `IRawElementProviderFragmentRoot`, and each published
element answers `IRawElementProviderFragment`: navigation, `GetRuntimeId` as a
safe array, `get_BoundingRectangle`, and hit testing from a screen point.

`SetFocus` returns `UIA_E_NOTSUPPORTED`. Moving focus is an action, and this
provider performs none. `GetFocus` is added separately in Slice 5, using the
host's copied focus snapshot rather than guessing or reading live view state.

The published tree is **flat**, and groups are filtered out of it. A container
whose children sit beside it rather than inside it would be announced as an
empty thing to step through, which is worse than not publishing it. Hierarchy,
and with it meaningful grouping, is deferred.

**Slice 3 — verification. Done.** Narrator announces the surface's elements with
their names and roles, and every published property has been cross-checked
against the table above. See below.

**Slice 4 — bounded button invocation. Implemented; manual activation check
pending.** An enabled authenticated-session button exposes `IInvokeProvider`. Its
`Invoke` implementation writes only a revision-bound semantic candidate into
the same bounded mailbox as native pointer and keyboard activation (Decision
0069). It does not move focus, synthesize a Windows input message, or call an
application. A full mailbox fails without adding a queue. The unit and host
tests prove the role/enabled/session gates and the exact candidate route; the
manual check below must still prove Narrator can activate a button.

**Slice 5 — focus reporting. Implemented; manual focus check pending.** The
provider returns the matching child from `GetFocus` and sets
`HasKeyboardFocus` only on that element, using one immutable snapshot of the
host's existing layout-validated focus (Decision 0070). It does not implement
`SetFocus` or send focus-change events.

**Slice 6 — read-only field values. Implemented; manual value check pending.**
A matching visible `Edit` exposes `IValueProvider`, returns its copied host
value, and is read-only to automation (Decision 0071). It has no `SetValue`,
caret, selection, text range, or value-change event.

Also deferred, each needing its own contract and decision: automation events and
live announcements, focus changes **raised** to assistive technology, focus
control, selection and caret reporting, text patterns and ranges, relations
between nodes, automation editing, and non-Windows accessibility adapters.

## Verification

Automated tests cover the mapping: every role's control type and focusability,
each property's source, empty and named nodes, rectangle conversion at several
scales and origins, an empty rectangle staying empty, and runtime-ID shape and
uniqueness within a snapshot.

The mapping is pure, so those tests need no window and no assistive technology.

Provider tests cover the COM object without Windows: the interfaces it answers
and the ones it refuses, a refused query clearing its output, every method
rejecting a null output rather than writing through it, reference counting
freeing the object exactly once, a panicking body returning a failure code
instead of unwinding, the Invoke gate admitting only an enabled authenticated-
session button to the revision-bound mailbox, and the Value gate returning only
a matching field snapshot while refusing every automation write.

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

To repeat the reading check:

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
- **no element supplied Invoke on that UI Lab run**, which is correct because
  the Lab has no authenticated event mailbox. A visible field now separately
  supplies its read-only Value pattern as Decision 0071 defines.

The window root reports two patterns, `Window` and `Transform`. Those come from
the host provider Windows supplies for the `HWND` itself — minimising, moving,
and resizing a top-level window are the system's business, not this provider's.
Anodrel's own window root supplies no pattern. Its enabled authenticated-session
buttons may supply only Invoke, as the contract above defines.

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

### Manual Invoke verification

The reading check uses `--ui-lab`, which is deliberately a local diagnostic and
has no authenticated event mailbox. Use the development UI Session Lab instead:

1. Run `npm run build` and then the `--sample-ui-client` command in
   `docs/DEVELOPMENT.md`.
2. Wait for the sample's authenticated document to replace the waiting screen,
   then start Narrator and move to its **Continue** button.
3. Activate it with Narrator's normal activation command. The sample must
   complete its `ui.events.read` round trip and close the session window, just
   as it does after a local click or Tab+Enter.
4. Repeat against a disabled button when one is present: it must be announced
   as unavailable and must expose no Invoke pattern in Inspect.

This check is currently **pending**. A passing result proves that a person can
activate a published button; it must not be inferred from the unit tests or a
client merely seeing the pattern.

### Manual focus verification

On the same development UI Session Lab surface, use Tab to reach a visible
field or button. Inspect must show that element's `HasKeyboardFocus` as true
and `GetFocus` must return that same element. Move focus with Tab again and ask
Inspect to refresh: the new provider snapshot must name the new target. The
previous provider is allowed to retain its old snapshot, because this slice
does not raise focus-change events. Confirm that a UI Automation client cannot
move focus through `SetFocus`.

This check is currently **pending**. It proves the screen-reader-visible focus
matches the host focus ring; it does not turn UI Automation into a focus-control
path.

### Manual field-value verification

On the development UI Session Lab, type an ordinary test value into a visible
field. Inspect must show the matching `Edit` exposes `Value`, its exact current
text, and `IsReadOnly = true`. Attempting `SetValue` through a UI Automation
client must fail and must leave the host-rendered text unchanged. Type another
character locally, refresh Inspect, and confirm a fresh provider reports the
new text without any value-change event. Repeat on a disabled field when one is
present: its visible value may be read, but `IsEnabled` must remain false.

This check is currently **pending**. It proves a screen reader can read what a
person entered without making UI Automation a writer or exposing typing to the
application.

See `docs/UI.md`, Decision 0026, and Decision 0063.
