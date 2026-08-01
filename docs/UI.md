# Anodrel native UI foundation v1

**Status:** Foundation contract. `anodrel-ui` provides an owned in-memory view
tree, deterministic layout, clipping, semantic action hit testing, a visible
accessibility snapshot, and portable focus traversal. It does not yet accept an
application package, protocol, script, renderer, native bridge, or
operating-system operation.

## Purpose and boundary

Anodrel needs a path beyond a host-rendered text document without importing a
browser, webview, or framework runtime. The first step is not a full UI toolkit:
it is a small portable model that every future host renderer can interpret in
the same way.

The model carries presentation and a semantic action identity only. An action
hit does not call the operating system, open a window, send a protocol message,
or grant a capability. A later application-session contract may decide how an
authenticated application receives it and must separately document permissions,
back-pressure, cancellation, focus, accessibility, and lifecycle behavior.

## Tree model

`UiDocument` owns one validated root node. Its in-memory Rust model has four
node kinds:

| Node | Fields | Meaning |
| --- | --- | --- |
| `Stack` | element ID, vertical or horizontal axis, padding, gap, semantic surface tone, children | Places child nodes in source order. |
| `Scroll` | element ID, one child | A vertical viewport that clips and translates its one child. |
| `Text` | element ID, plain text, font size, semantic text tone | A non-interactive text run. |
| `Action` | element ID, plain label, font size, enabled state, semantic action tone | A semantic, hit-testable action. |

An element ID is 1–64 ASCII bytes containing letters, digits, `.`, `_`, or `-`;
it starts and ends with a letter or digit. IDs must be unique throughout a
document. Text and labels are non-empty, bounded single-line UTF-8 without control
characters. An `Action` uses its element ID as the semantic event identity;
there is no second command or native operation field.

The foundation accepts at most 512 nodes, depth 32, 32 KiB of text and labels
combined, a font size from 8 through 96 logical pixels, and padding or gaps no
larger than 256 logical pixels. These are validation limits, not layout hints.

## Appearance roles

Nodes may request a small semantic appearance role: a stack is `Plain` or
`Raised`, text is `Primary`, `Secondary`, or `Accent`, and an action is
`Neutral` or `Accent`. Constructors choose the least surprising defaults:
plain stacks, primary text, and neutral actions. The `with_surface_tone` and
`with_tone` builders let a host-owned document make an explicit request.

These roles are deliberately not a theme engine. They contain no colour,
font family, size, pixel metric, image, shader, renderer handle, or operating-
system value. They do not change validation, measurement, layout, clipping,
accessible semantics, focus order, enabled state, or a semantic action ID. A
host renderer maps the roles to its own palette and drawing rules, so the same
document does not depend on special element-ID strings or a particular host's
visual identity.

## Layout and input

The host supplies a text measurer because font shaping belongs to the operating
system. Given a host client rectangle, `anodrel-ui` lays out the validated tree
deterministically:

- a vertical stack places children from top to bottom; a horizontal stack
  places them left to right;
- a text node takes its measured size, bounded by its stack's available area;
- an action receives its label's measured size plus fixed owned padding, with a
  minimum 36-pixel height; stacks and actions stretch across the available
  cross axis;
- stack children are clipped to every ancestor's content rectangle; and
- a hit test checks visible enabled actions in reverse paint order and returns
  `UiEvent::ActionInvoked(element_id)` only.

A `Scroll` is one vertical viewport. The host retains a `UiScrollState` for
each scroll element ID and passes those states to
`layout_with_scroll_offsets`. Layout never changes caller-owned state: it
clamps the supplied position against the current full child height and viewport
height, translates only the child vertically, clips it on all four viewport
edges, and returns `UiScrollMetrics` so the host can retain a clamped value for
the next pass. `layout` remains the zero-offset convenience method. The model
has no wheel or gesture input, scrollbar, horizontal scrolling, overscroll,
inertia, wrapping, transforms, z-index, animation, pointer capture, text
editing, or implicit native behavior. See `docs/SCROLLING.md` and Decision
0038.

Every visible `UiLayoutItem` carries both clipped `bounds` and un-clipped
`paint_bounds`. Renderers draw the latter then clip their own output to the
former; hit testing, focus, and accessibility use only `bounds`. This preserves
the original shape and text position of an item that is only partly visible in
a scroll viewport.

## Focus traversal

`UiFocus` keeps one optional focus target for a specific `UiLayout`. Its
`move_next` and `move_previous` methods traverse visible enabled actions in
source order and wrap at each end. Text, stacks, disabled actions, and fully
clipped actions cannot receive focus. If the current target disappeared after a
relayout, traversal starts at the appropriate end of the new layout.

`activate(layout)` returns `UiEvent::ActionInvoked(element_id)` only when the
current target is still a visible enabled action in that layout. It has the same
semantic-only meaning as a pointer hit: no operating-system work, process
launch, protocol message, or capability follows from this method. The type does
not observe raw keyboard events, set an operating-system focus handle, draw a
focus ring, or provide text editing. A host must deliberately map its keyboard
and accessibility lifecycle to this portable state.

## Accessibility semantics

`UiDocument::accessibility_snapshot(layout)` returns the visible elements of a
specific layout pass in source order. Every entry has the stable element ID,
clipped logical-pixel bounds, role, enabled state, and, for text or actions, a
plain-text accessible name.

| UI node | Accessibility role | Accessible name | Enabled |
| --- | --- | --- | --- |
| `Stack` | `Group` | none | false |
| `Scroll` | `Group` | none | false |
| `Text` | `StaticText` | text value | false |
| `Action` | `Button` | action label | action enabled state |

The snapshot contains no invisible or fully clipped node. It does not expose a
native UI Automation, AT-SPI, NSAccessibility, or Assistive Technology Service
API; it does not set focus, manage keyboard navigation, make announcements, or
invoke an action. A future operating-system accessibility adapter must consume
this bounded snapshot through its own documented lifecycle and permission
boundary.

## Compatibility

This is a Rust API foundation, not an application file or protocol format.
`docs/UI_DOCUMENTS.md` separately defines the exact capability-free JSON form.
Its version 1 shape represents only stacks, text, and actions; it deliberately
rejects an in-memory scroll node until a new exact format exists. The Windows
UI Lab uses one compiled-in fixture and the separate explicit developer preview
can render one bounded operator-selected document. When a package or session
transports this tree to a host, that surface must still have its own lifecycle,
resource limits, compatibility tests, and security decision before reuse.

## Windows UI Lab

The direct Windows host includes a fixed, host-owned consumer of this contract:

~~~text
anodrel-windows-host --ui-lab
~~~

It uses the Windows text-measurement seam, Anodrel's software canvas, and a
validated `UiDocument` to draw a responsive native screen. Its raised action
group, text prominence, and emphasized action come from the document's
semantic appearance roles; the renderer does not infer them from element IDs.
Hovering and
clicking an action exercises the same layout hit test and displays its semantic
element ID. Tab and Shift+Tab exercise the portable focus order with a visible
focus ring; Enter activates only that same semantic action. The host-owned Lab
also places its compiled v1 fixture inside an in-memory scroll viewport and
adds local diagnostic exercises. Page Up and Page Down move only that retained
viewport state. The view has no package input and every event changes only its
own diagnostic reading or viewport position: it does not open a process, read
a file, send a protocol message, or grant a capability. It is a renderer-and-
input test, not an application UI API.

## Verification

The portable crate tests ID validation and every document resource limit,
unique IDs, vertical and horizontal placement, clipping, responsive bounds,
disabled actions, top-most action hit testing, appearance-role defaults and
selection, accessibility role/name/visibility semantics, and focus
traversal/activation. It also tests finite, deterministic line, page, absolute,
and relayout clamping for the independent scroll-state foundation, plus viewport
translation, clipping, metrics, and stale input clamping for scroll containers.
It has no operating-system or third-party runtime dependency. The Windows host additionally tests
that the UI Lab paints content, resolves every fixed action to its own ID,
tracks scaled hit testing, and changes only host-owned diagnostic state on
invocation.
