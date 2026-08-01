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

`UiDocument` owns one validated root node. Version 1 has only three node kinds:

| Node | Fields | Meaning |
| --- | --- | --- |
| `Stack` | element ID, vertical or horizontal axis, padding, gap, children | Places child nodes in source order. |
| `Text` | element ID, plain text, font size | A non-interactive text run. |
| `Action` | element ID, plain label, font size, enabled state | A semantic, hit-testable action. |

An element ID is 1–64 ASCII bytes containing letters, digits, `.`, `_`, or `-`;
it starts and ends with a letter or digit. IDs must be unique throughout a
document. Text and labels are non-empty, bounded single-line UTF-8 without control
characters. An `Action` uses its element ID as the semantic event identity;
there is no second command or native operation field.

The foundation accepts at most 512 nodes, depth 32, 32 KiB of text and labels
combined, a font size from 8 through 96 logical pixels, and padding or gaps no
larger than 256 logical pixels. These are validation limits, not layout hints.

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

The model has no scrolling, wrapping, transforms, z-index, animation, pointer
capture, text editing, or implicit native behavior. A future version needs a
new documented contract before adding any of them.

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
| `Text` | `StaticText` | text value | false |
| `Action` | `Button` | action label | action enabled state |

The snapshot contains no invisible or fully clipped node. It does not expose a
native UI Automation, AT-SPI, NSAccessibility, or Assistive Technology Service
API; it does not set focus, manage keyboard navigation, make announcements, or
invoke an action. A future operating-system accessibility adapter must consume
this bounded snapshot through its own documented lifecycle and permission
boundary.

## Compatibility

This is a Rust API foundation, not an application file or protocol format. No
untrusted source can construct a document through Anodrel today. When a package
or session transports this tree, that surface must have its own version,
resource limits, compatibility tests, and security decision before reuse.

## Windows UI Lab

The direct Windows host includes a fixed, host-owned consumer of this contract:

~~~text
anodrel-windows-host --ui-lab
~~~

It uses the Windows text-measurement seam, Anodrel's software canvas, and a
validated `UiDocument` to draw a responsive native screen. Hovering and
clicking an action exercises the same layout hit test and displays its semantic
element ID. Tab and Shift+Tab exercise the portable focus order with a visible
focus ring; Enter activates only that same semantic action. The view has no
package input and every event changes only its own diagnostic reading: it does
not open a process, read a file, send a protocol message, or grant a capability.
It is a renderer-and-input test, not an application UI API.

## Verification

The portable crate tests ID validation and every document resource limit,
unique IDs, vertical and horizontal placement, clipping, responsive bounds,
disabled actions, top-most action hit testing, accessibility role/name/
visibility semantics, and focus traversal/activation. It has no operating-
system or third-party runtime dependency. The Windows host additionally tests
that the UI Lab paints content, resolves every fixed action to its own ID,
tracks scaled hit testing, and changes only host-owned diagnostic state on
invocation.
