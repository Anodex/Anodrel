# Native session context menus

**Status:** Accepted Protocol 1.32 contract; implementation pending. This is
not yet an available operation or installed-application grant.

## Purpose

Anodrel's first context menu will be a host-owned local popup for semantic
commands in one authenticated session's primary native view. It is a bounded
desktop command surface, not a browser context-menu API, coordinate API,
selection API, native menu bridge, or callback mechanism.

This feature deliberately does not migrate Anodex's existing browser-backed
menu. That menu derives choices from browser text selection and link data;
those values are neither present in nor appropriate for Anodrel's native UI
contract.

## Planned public boundary

Protocol 1.32 will add `menu.context.replace`, requiring a host-issued
`menu.context.write` capability. Its payload will be exactly:

~~~json
{
  "items": [
    { "id": "document.rename", "label": "Rename", "enabled": true },
    { "id": "document.archive", "label": "Archive", "enabled": false }
  ]
}
~~~

The model contains one through sixteen items. Each item has exactly three
fields:

| Field | Rule |
| --- | --- |
| `id` | A unique one-to-64-byte ASCII semantic ID. It starts and ends with an alphanumeric character and otherwise uses only letters, digits, `.`, `_`, or `-`. |
| `label` | One to 96 UTF-8 bytes with no control character. It is display text only. |
| `enabled` | An explicit Boolean. Disabled items never create an event. |

Unknown, missing, duplicate, wrongly typed, or extra fields are invalid. The
encoded payload must be at most 8 KiB as well as fitting the normal 64 KiB
protocol frame. It supplies no separator, submenu, icon, check state, shortcut,
native command ID, window, view, document node, selection, link, URL,
coordinate, callback, data payload, or executable action.

On success the result will be exactly one host-owned decimal revision:

~~~json
{ "revision": "1" }
~~~

It is not a native menu identifier, handle, window ID, position, or persistent
preference. A host with no attached context-menu surface will return
`menu.unavailable` and retain its previous model.

## Local activation and delivery

The direct Windows host will react only to a pointer-originated
`WM_CONTEXTMENU` message for the authenticated session's primary native view.
Windows selects the popup's screen position from that message; the position
never enters a request, response, event, log, or callback. Keyboard-originated
context-menu messages, a secondary view, and every programmatic request remain
outside this first contract.

The UI thread creates the native popup and its private numeric command mapping.
Choosing a current enabled item offers one revision-bound candidate to the same
fixed ordered interaction mailbox used by document actions and the menu bar.
No selection calls application code, runs a native operation, or creates a
separate queue.

The existing granted `ui.events.read` operation will deliver a revalidated
candidate in this event shape:

~~~json
{
  "eventName": "menu.context.action.invoked",
  "source": "native.context_menu",
  "schemaVersion": { "major": 1, "minor": 32 },
  "payload": { "contextMenuRevision": "1", "action": "document.rename" }
}
~~~

The event carries no pointer position, timing, hover state, trigger target,
selection, URL, menu handle, command number, keyboard state, open/dismiss
event, or proof that someone saw the menu. A model that is replaced, removed,
or disables the chosen action before the next event read is discarded by the
core. The existing `dropped` and `discarded` counts cover it.

## Host ownership and compatibility

The authenticated worker transfers each validated replacement through one
per-session `ContextMenuMailbox` and waits no more than five seconds for the
owning UI thread. The mailbox retains at most one request and clears a timed
out slot by identity, so a late completion cannot affect a later model.

The host retains the active portable model and every native object. An
installed record at version 1.19 will be the first permitted to name
`menu.context.write`; it will be a strict superset of version 1.18. Older
records or protocol versions cannot name the grant or operation.

The first implementation has no keyboard invocation, separator, submenu,
radio/check state, icon, shortcut, dynamic enablement, opening callback,
selection or link fact, document target, coordinate, menu-state readback,
secondary view, persistent configuration, non-Windows adapter, or browser
integration.

See Decision 0120, `docs/MENUS.md`, and `docs/THREAT_MODEL.md`.
