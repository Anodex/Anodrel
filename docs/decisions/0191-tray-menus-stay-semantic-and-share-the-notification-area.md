# Decision 0191: Tray menus stay semantic and share the notification area

**Status:** Accepted

**Date:** 2026-09-03

## Context

A native system-tray surface is an important desktop capability, but a broad
tray API would put persistent desktop presence, arbitrary icon files, native
handles, pointer coordinates, callbacks, and background lifetime under
application control. That is a much wider authority boundary than an Anodrel
session should have.

Windows uses the same `Shell_NotifyIconW` resource for the current notification
balloon and any tray interaction. A second tray icon would make a host look
broken and create independent cleanup lifetimes. The notification-area resource
has therefore been separated in Decision 0190 before this capability begins.

## Decision

Protocol 1.33 adds `tray.replace`, protected by the separate `tray.write`
capability. Its payload is exactly a complete bounded list of semantic menu
items: action ID, display label, and enabled state. It uses the existing
context-menu item grammar and limits: one through sixteen items, unique action
IDs, no submenu, separator, shortcut, image, icon, tooltip, callback, native
command ID, position, coordinate, or input value.

The host applies the complete model through one bounded UI-thread bridge and
retains a monotonic `trayRevision`. A successful replacement makes one
host-created notification-area entry available for the session. Its icon and
tooltip are host-selected; an application cannot replace, hide, inspect, or
create an icon.

A local right-click opens the current native popup at the position Windows
supplies. The host maps its private command number back to one semantic action,
then the core revalidates that action against the current enabled model and
revision. `ui.events.read` may return only:

~~~json
{
  "eventName": "tray.action.invoked",
  "source": "native.tray",
  "schemaVersion": { "major": 1, "minor": 33 },
  "payload": { "trayRevision": "1", "action": "application.command" }
}
~~~

A left click asks Windows to foreground only the session's own main window.
The application receives no focus, activation, visibility, click, coordinates,
or delivery result. It cannot turn a closed window into a background process.

The native entry is shared with notifications. Whichever capability first
requires it creates it on the owning UI thread; the other reuses that entry.
Dropping the session removes the one entry. A tray operation never creates a
second icon.

## Consequences

- Anodrel gains a direct native tray menu without a webview, desktop framework,
  or application-controlled native resource.
- Menu command routing is revision-checked and shares the established bounded
  event pull instead of adding a callback channel.
- One notification-area icon and one cleanup lifetime serve notifications and
  tray behavior together.
- The first slice works only while the host's session window exists. It is not
  close-to-tray behavior or general background execution.

## Revisit conditions

Revisit for product-selected tooltip text, submenus, separators, checked
states, tray notification badges, a background host lifecycle, a user-visible
window toggle, installation identity, non-Windows implementations, or a real
application requirement that cannot use this bounded semantic menu.
