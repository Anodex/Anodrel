# Anodrel Windows accessibility verification

## Verification

`--uia-property-probe` is the repeatable host-only property, raw-tree,
control-view, fixed-geometry, and fixed-Value-pattern check for the UI Lab. It complements the
manual checks below; it does not replace Narrator's spoken-output or Inspect's
highlight verification. See
`docs/UI_AUTOMATION_PROBE.md` and Decisions 0106, 0107, and 0108.

### Automated UI Lab property/tree/geometry/Value-pattern acceptance

This passed on Windows on 2026-08-24. The direct first-party client created a
separate MTA apartment, attached to the temporary UI Lab through its real
`HWND`, and read each fixed node through Windows UI Automation. It confirmed
the Anodrel window root, Windows' expected native `TitleBar` peer, and all
twenty-three Anodrel semantic document nodes in their fixed parent and sibling
order in both raw and control views. For every Anodrel node, it compared
`Name`, `AutomationId`, and `ControlType` with the compiled UI Lab contract.

It also read the root and fixed `ui.lab.field` bounding rectangles, confirmed
that both were non-empty and the field lay inside the window, then used
Windows' desktop-level `ElementFromPoint` at the field centre and received the
same AutomationId. Because that API answers for the topmost desktop element,
the temporary test window is placed in the topmost band for this short
diagnostic only and is destroyed afterward. The check therefore catches both
bad published geometry and a broken provider hit-test route without changing a
product window or accepting a caller-selected coordinate.

For that one fixed field, the probe also obtained Windows' client-side
`IUIAutomationValuePattern` for the provider-side read-only Value pattern. It
confirmed the compiled empty initial value and `IsReadOnly = true`; the returned
`BSTR` was copied and released inside the private worker. This establishes the
real client/provider pattern bridge without reading a person's text.

It intentionally did **not** call any interactive pattern, look up focus, or
register an event handler. Its only field-text read is the compiled empty Value
check described above. Arbitrary geometry, visible highlight placement, and
interactive behavior remain distinct acceptance concerns.
Re-run it with the command in
`docs/UI_AUTOMATION_PROBE.md`; a pass proves this exact property/tree boundary,
including control-view navigation and one fixed hit-test target, not spoken
output or visual highlight geometry.

Automated tests cover the mapping: every role's control type and focusability,
each property's source, empty and named nodes, rectangle conversion at several
scales and origins, an empty rectangle staying empty, and runtime-ID shape and
uniqueness within a snapshot.

The mapping is pure, so those tests need no window and no assistive technology.
Hierarchy tests also prove preorder parentage, direct child and sibling
navigation, top-level root ownership, group pattern/focus refusal, and deepest
hit testing without COM or a native window.

Provider tests cover the COM object without Windows: the interfaces it answers
and the ones it refuses, a refused query clearing its output, every method
rejecting a null output rather than writing through it, reference counting
freeing the object exactly once, a panicking body returning a failure code
instead of unwinding, the Invoke gate admitting only an enabled visible
authenticated-session button to the revision-bound mailbox, and the Value gate returning only
a matching field snapshot while refusing every automation write, and that every
pattern response retains the provider's canonical COM identity before Windows
queries its requested pattern interface. They also
prove the Focus gate admits only a visible enabled field or button, keeps one
provider's updated focus snapshot local to that provider, and refuses an
expired, busy, unknown, or late-completing route without changing host focus.
The event adapter separately proves that an empty publication has no event
source and that a focus event names the currently published focused child.
The scroll provider tests additionally prove that only the selected overflowing
Group answers the standard interface and pattern, reports finite vertical
values, accepts only closed vertical commands, rejects malformed or horizontal
requests, and never lets a busy or timed-out route apply later. The scroll-item
tests prove a fully clipped eligible descendant answers only the standard
ScrollItem interface and offers its fixed semantic ID through that same route.
The Windows-host tests prove line, page, percentage, and item-reveal commands
reach the same retained position used by direct pointer, wheel, and keyboard
movement while excluding nested contents.

### Confirmed against real UI Automation before hierarchy

The original flat slices were queried by a real UI Automation client —
`UIAutomationClient` driving `FindFirst` and `FindAll` against a running
`--ui-lab` window. The window reports `AutomationId = anodrel.surface`, which
Windows' default window provider leaves empty, so the host's own provider was
accepted, `QueryInterface` succeeded, and `GetPropertyValue` returned a
correctly read `BSTR`.

Walking its children returned all eleven then-published elements with their
control types, names, automation IDs, enabled and focusable state, and screen
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

This was run and **passed** on Windows 11 for the pre-hierarchy provider:
Narrator announced each flat-surface element with its name and role. It also
earned its keep — see the note after step 7.

The hierarchy-specific check below is still pending; passing the earlier flat
reading check does not prove grouping is announced or navigated correctly.

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
5. Confirm a disabled action is announced as unavailable. Scroll-item behavior
   has its own check below because a clipped semantic node is now deliberately
   navigable rather than omitted.
6. Optionally cross-check with **Accessibility Insights for Windows** or the
   Windows SDK's **Inspect** tool, which show the raw UI Automation tree
   including `AutomationId`, `ControlType`, `IsEnabled`, `IsOffscreen`, and
   `BoundingRectangle`. Confirm each matches the mapping table above and that
   a visible element's highlighted rectangle sits over it on screen.
7. Close Narrator with `Ctrl+Windows+Enter`.

If nothing is announced, check in this order: that Narrator is actually running,
that the Anodrel window has focus, and only then suspect the provider. The
repeatable property probe already walks the control view, which is the view
Narrator navigates, but it cannot establish that Narrator spoke the result.

### Manual hierarchy verification

Run a nested `--ui-lab` document after this slice is available. In Inspect or
Accessibility Insights, expand the host window and confirm that a visible
`Group` contains only its direct document children, nested groups contain their
own children, and a child reports that exact group as `Parent`. Walk with
Narrator's item navigation and confirm it enters and leaves those groups without
skipping their children. At a point inside a child, Inspect's highlight and
`AutomationElement.FromPoint` must choose the child rather than its containing
group.

This check is **pending**. It validates real client tree navigation and spoken
grouping; unit tests cannot establish either.

### Manual scroll-item verification

Run the version 2 scrolling UI Session Lab, then find an action initially below
the viewport in Inspect or Accessibility Insights. Before movement, it must stay
in the tree with `IsOffscreen=true`, an empty bounding rectangle, and
`ScrollItemPattern`; it must not offer Invoke or move focus. Call
`ScrollIntoView` through the tool or a UI Automation client. The viewport must
move just enough to show the item, a fresh provider must report a non-empty
rectangle and `IsOffscreen=false`, and no action, focus change, value read, or
application event may occur. Repeat for an already visible item (successful
with no movement) and for an item inside a nested viewport (no outer item
pattern).

This check is **pending**. It verifies real Windows tree navigation and spoken
scroll behavior; the focused tests do not substitute for it.

### Inspect cross-check before hierarchy

Before hierarchy, the Windows SDK's **Inspect** tool checked every then-
published `--ui-lab` element against the mapping table. All eleven passed with
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
   `docs/DEVELOPMENT_DIAGNOSTICS.md`.
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
does not make earlier providers chase live state. A UI Automation client
registered for focus-change events must receive one event naming the new target
after Tab, pointer focus, and a changed successful `SetFocus`; repeating focus
on the same control and attempting disabled or clipped targets must produce
none. Confirm that `SetFocus` does not activate an action or edit a field.

This check is currently **pending**. It proves the screen-reader-visible focus
matches the host focus ring and that controlled focus stays inside the declared
boundary; it does not expose focus to an application or turn UI Automation into
a general input path.

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
