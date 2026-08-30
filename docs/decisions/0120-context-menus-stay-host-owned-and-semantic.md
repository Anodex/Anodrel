# Decision 0120: Context menus stay host-owned and semantic

**Status:** Accepted and implemented in the direct Windows host and both
first-party SDKs.

**Date:** 2026-08-29

## Context

The first native session menu intentionally provides only a top-level menu bar.
Anodex also has a browser-backed context menu, but that implementation receives
browser selection state, link URLs, renderer coordinates, and callbacks. None
of those facts exist in Anodrel's owned native UI, and importing them would
turn a reusable platform feature into an Electron compatibility shim.

Applications still need a familiar local command surface. Letting one request
name a screen point, native handle, document selector, selection, URL, menu
handle, callback, or operating-system command would expose either native window
authority or private user input. A free-form context menu must not become a
back-channel for browser facts or pointer telemetry.

## Decision

Protocol 1.32 adds `menu.context.replace`, protected by
the separate `menu.context.write` capability. It accepts one exact complete
model of one through sixteen items. Every item has only a bounded semantic ID,
display label, and explicit enabled flag.

The direct Windows host shows that model only when it receives a normal
pointer-originated `WM_CONTEXTMENU` for the requesting session's primary native
view. The host keeps the coordinate, native popup handle, private numeric
command mapping, menu opening and dismissal, and command selection entirely on
the UI thread. It will not accept a caller-provided point or offer a keyboard,
selection, link, document-node, browser, callback, or native-handle route in
this first slice.

An enabled selection becomes one revision-bound semantic candidate in the
existing bounded interaction mailbox. The existing `ui.events.read` operation
revalidates it before returning `menu.context.action.invoked`. That event
will include only an opaque context-menu revision and the semantic action ID.
It will not report position, pointer state, opening, dismissal, selection,
keyboard state, command number, or evidence that a person saw the popup.

The portable core commits a replacement only after the host-owned UI-thread
bridge accepts it. An unavailable, timed-out, or failed bridge retains the
previous model and returns the existing safe `menu.unavailable` category.

## Consequences

- Anodrel gains a reusable native popup command surface without a browser
  runtime or Electron-specific data model.
- Anodex's current browser context menu is not claimed to be migrated: link,
  text-selection, renderer-coordinate, and action-callback behavior remain
  application migration work.
- The first context menu is primary-view-only and pointer-originated. Keyboard
  activation, secondary-view menus, separators, submenus, check state,
  shortcuts, dynamic enablement, and contextual payloads remain separate
  decisions.

## Alternatives considered

**Expose the browser's context-menu parameters.** They are browser-runtime
facts, include sensitive selection and URL data, and do not generalize to an
owned native UI. Refused.

**Let applications request a popup at coordinates they supply.** It would
reveal and control native placement while allowing popup spam unrelated to a
local user gesture. Refused.

**Reuse the top-level `menu.replace` model and grant.** A menu bar and a local
popup have different trigger, presentation, and future accessibility policy.
Keeping their models and grants separate prevents a menu-bar permission from
silently acquiring a new surface. Refused.

## Revisit conditions

Revisit before adding keyboard invocation, a target element, a selection or
link value, a coordinate, separator, submenu, check or radio state, shortcut,
menu callback, result readback, secondary-view route, non-Windows adapter, or
any browser integration. Each changes authority, privacy, or lifecycle and
needs its own contract, threat-model entry, and verification.
