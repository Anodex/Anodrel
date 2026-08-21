# Anodrel UI Automation focus

**Status:** Implemented on Windows; manual screen-reader verification is still
required.**

## Purpose

Anodrel's custom-drawn UI exposes a host-owned keyboard focus ring. Local Tab,
pointer, and keyboard input already move that focus through visible enabled
fields and actions. This document defines the matching Windows UI Automation
route so assistive technology can ask the same fragment to focus one of those
controls.

It is not a public application focus API. Applications cannot request, inspect,
subscribe to, or learn about focus. See `docs/ACCESSIBILITY.md` and Decision
0073.

## Exact operation

Only an `IRawElementProviderFragment` for a visible, enabled `Edit` or `Button`
may succeed from `SetFocus`.

The provider's immutable snapshot must contain the element and its route must
match the current host view:

| Provider | Host check before focus changes |
| --- | --- |
| Authenticated UI session | The route revision equals the session's current accepted document revision, and the target remains visible, enabled, and focusable in the current layout. |
| Host UI Lab | The target remains visible, enabled, and focusable in its fixed host-owned layout. |

The root, text, groups, disabled elements, fully clipped elements, malformed
IDs, stale session providers, and unavailable views fail. A focus request for
the control already focused succeeds without moving anything.

The Microsoft UI Automation framework focuses the parent fragment before it
calls `SetFocus`. Anodrel therefore changes only its internal focus state. It
does not activate or foreground a window, send a Windows input message, invoke
an action, edit a value, or make an application callback.

## Threading and lifetime

The provider is created from `WM_GETOBJECT` on the owner UI thread. A later
automation method can arrive from a UI Automation caller, so it cannot borrow
or mutate the native view directly.

Instead, one focus request is stored in a private per-window route and wakes
the owner with a private **payload-free** window message. The message carries
no pointer, handle, element ID, or application text. The owner takes only its
own pending request, revalidates it against the live layout, updates focus, and
completes the route. A caller waits at most 250 milliseconds; timing out clears
that exact request before the host applies focus, and a late completion is
ignored.

This gives the UI thread sole authority to write focus while allowing a screen
reader to receive an honest `S_OK` only after the host accepted the target.
A genuine move then raises one separate host-only focus-change event; see
`docs/UI_AUTOMATION_EVENTS.md`. A successful no-op does not raise one.

## Boundaries

- No protocol version, operation, grant, installed-record field, or SDK method
  exists for this feature.
- A route cannot name a different window or session.
- The request contains no native input, coordinates, focus state, caret,
  selection, text, callback, or event.
- An enabled button is only focused. It still invokes only through the existing
  bounded `Invoke` route, and a field remains read-only to UI Automation.
- A successful request updates the provider snapshot that made it so an
  immediate `GetFocus` query on that provider stays truthful. Older providers
  do not read arbitrary later focus changes.

## Verification

Automated tests cover the one-request route, busy and timeout recovery, the
snapshot revision gate, the focusability gate, the COM result, and session
isolation.

To verify with Narrator on Windows:

1. Run `npm run build`, then use the `--sample-ui-client` command in
   `docs/DEVELOPMENT.md` to open the authenticated UI Session Lab.
2. Start Narrator and move to a visible field or button in the Anodrel window.
3. Ask Narrator to set focus on that element, then type in a field or activate
   a button using Narrator's normal command.
4. Confirm the native focus ring moves to the announced element and that
   keyboard input reaches that element. A screen reader must not move focus to
   a disabled or clipped control.

Record the result in `docs/ACCESSIBILITY.md`. The automated route proves the
boundary; only this check proves it is usable with a real assistive technology.
