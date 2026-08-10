# Decision 0063: Windows accessibility maps the owned snapshot, one direction only

**Status:** Accepted

**Date:** 2026-08-09

## Context

Decision 0026 established a portable accessibility snapshot: roles, names,
enabled state, and clipped bounds derived from a validated document and one
concrete layout. It deliberately stopped short of any operating-system adapter,
and `ROADMAP.md` still lists operating-system accessibility adapters as a
separate Phase 2 gate.

Making an Anodrel surface readable by a screen reader now needs two choices: the
Windows accessibility API to target, and how much of the way to go in one step.

Windows offers two. **Microsoft Active Accessibility** is the older `IAccessible`
model; it is widely supported but its role and state vocabulary is fixed, coarse,
and awkward to extend. **UI Automation** is the current model, is what Narrator
and modern tooling prefer, has a control-type and property vocabulary that maps
cleanly onto the roles the snapshot already has, and is inspectable with
first-party Microsoft tools. Both are COM.

The second choice matters more. A usable provider needs COM interfaces, tree
navigation, `WM_GETOBJECT` handling, and marshalling of variants and safe
arrays — hand-written, because Anodrel ships no third-party runtime crates.
Doing all of that at once would land a large body of unsafe code whose
correctness could only be judged by running a screen reader against it.

## Decision

Target **UI Automation**, and split the work so the part that can be tested
without a window is built and proved first.

`anodrel-windows-accessibility` is a pure mapping from one
`UiAccessibilitySnapshot` to the exact values UI Automation asks for: control
type IDs, the property values listed in `docs/ACCESSIBILITY.md`, runtime IDs,
and screen-space physical rectangles. It performs no operating-system call,
holds no lock, and cannot fail. The conversion from clipped logical bounds to
screen rectangles takes the client origin and scale as arguments rather than
querying them, so it is a pure function testable at any display density.

The boundary is **one direction only**. Nothing flows from Windows or from
assistive technology back to an application: no tree read, no focus query, no
announcement callback, and no way to detect that a screen reader is present.
Whether a user relies on assistive technology is not observable through this
boundary, for the same reason Decision 0062 refuses to report that a
notification was seen.

An application supplies a UI document and nothing else. It cannot pass a handle,
see a UI Automation identifier, register a provider, raise an event, force
focus, or override a mapping. There is no accessibility-specific field in the
document format, and a role the model cannot express is a gap in the model
rather than something to add here.

The UI Automation provider itself — the COM interfaces, `WM_GETOBJECT`
registration, and tree navigation — is explicitly the next step and is not part
of this decision beyond naming it.

## Consequences

Positive:

- accessibility semantics stay derived from the same validated document and
  layout the surface actually draws, so they cannot drift from what is on
  screen;
- the mapping is pure, so every role, property, density, and edge case is tested
  without a window or a screen reader;
- the remaining unsafe COM work has a small, settled, already-proved input; and
- an application gains no new authority and no new observation channel.

Tradeoffs:

- until the provider exists, an Anodrel window still exposes only the default
  accessibility Windows gives any top-level window, so **a screen reader cannot
  read an Anodrel surface yet**;
- runtime IDs are positional, so replacing a document invalidates them — correct
  for a replaced tree, but it means an assistive technology holding an old
  element sees it disappear rather than move; and
- choosing UI Automation over Active Accessibility means older assistive
  technologies that speak only `IAccessible` rely on the system bridge rather
  than on a native provider.

## Revisit conditions

Revisit when the provider needs automation events or live announcements, when an
action should be invocable through the `Invoke` pattern, when text patterns and
ranges are required, when relations between nodes are needed, when a role
appears that the portable model cannot express, or when a non-Windows
accessibility adapter needs an equivalent mapping.
