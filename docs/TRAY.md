# Native tray menus

**Status:** Protocol 1.33, its portable core boundary, typed Rust facade, and
direct Windows Shell32/User32 bridge are implemented. A real desktop
notification-area interaction remains the manual acceptance check.

## Purpose

Anodrel's tray capability provides one small native menu on the host's existing
notification-area icon. It is a semantic command surface, not an API for
creating or controlling Windows tray objects.

~~~text
application complete menu model
            |
            | tray.replace / tray.write
            v
session-owned revision and UI-thread mailbox
            |
            v
host-selected icon + host-created native popup
            |
            | person chooses an enabled item
            v
revision-checked tray.action.invoked event
~~~

The icon and its tooltip are selected by the host. A session shares that one
icon with [notifications](NOTIFICATIONS.md), so enabling a tray never creates a
second notification-area entry.

## Capability and operation

Installed application record version **1.24** adds the optional `tray.write`
grant. Earlier records naming it are invalid. A session must also already have
`ui.events.read` to receive a selected action.

Protocol **1.33** defines one operation:

~~~json
{
  "protocolVersion": { "major": 1, "minor": 33 },
  "kind": "request",
  "requestId": "…",
  "operation": "tray.replace",
  "payload": {
    "items": [
      { "id": "document.open", "label": "Open window", "enabled": true },
      { "id": "document.sync", "label": "Sync now", "enabled": false }
    ]
  }
}
~~~

The payload contains exactly `items`. It holds one through **16** items in
display order. Each item has exactly `id`, `label`, and `enabled`:

| Field | Rule |
| --- | --- |
| `id` | A unique ASCII semantic action ID: 1–64 bytes, alphanumeric at both ends, with only `.`, `_`, or `-` inside. |
| `label` | One nonempty, control-free display value of at most 96 UTF-8 bytes. |
| `enabled` | A Boolean. Disabled items are visible but cannot produce an event. |

There are no separators, submenus, shortcuts, checkmarks, images, badges,
tooltips, window targets, coordinates, native menu IDs, icon values, or click
handlers. A complete replacement is required; patching a prior model is not
supported.

On success, the response contains the monotonic nonzero revision:

~~~json
{ "revision": "1" }
~~~

## Events and local interaction

Only a person selecting a current enabled tray item can produce an event.
`ui.events.read` returns it in normal bounded delivery order:

~~~json
{
  "kind": "event",
  "eventName": "tray.action.invoked",
  "source": "native.tray",
  "protocolVersion": { "major": 1, "minor": 33 },
  "schemaVersion": { "major": 1, "minor": 33 },
  "payload": {
    "trayRevision": "1",
    "action": "document.open"
  }
}
~~~

The revision and action are revalidated after the native popup closes. A menu
that was replaced, an item disabled by the newer model, a cancelled popup, or a
private command outside the current model produces no application event.

Left-clicking the icon asks Windows to foreground only the session's own main
window. It carries no protocol event and no report of whether Windows accepted
the request. Right-click placement comes from Windows; applications supply no
pointer position or popup target.

## Errors

| Code | Meaning |
| --- | --- |
| `capability.denied` | The session lacks `tray.write`. |
| `request.payload_invalid` | The model is malformed, oversized, or carries an unsupported field. |
| `tray.unavailable` | The session has no host tray surface, its UI bridge could not apply the model, or Windows refused the native entry. |

No error contains a window handle, icon, pointer coordinate, command number,
native failure, or application-supplied text.

## Lifetime and limits

The tray appears only after a successful replacement and exists only while the
session's host window exists. Closing the window ends the session and removes
the icon; this first slice does not keep Anodrel alive in the background.

At most one model replacement can wait for the UI thread. It uses the same
five-second bounded handover discipline as native context menus. A failed
replacement retains the last accepted model. The host builds private Windows
menu objects before it commits the model, so native construction failure cannot
erase a working tray menu.

## Windows implementation and verification

The direct Windows host creates the existing host-selected Shell32 entry only
after a notification or accepted tray model needs it. A tray model configures
one private `WM_APP` callback on that same entry; it never adds another icon.
On a local right-button release, the host reads the current cursor position
only inside the callback, creates a temporary User32 popup, maps a selected
private command to its semantic action, and destroys the popup before the
callback returns. A local left-button release makes only a best-effort Windows
foreground request for the same session's main window.

The host's focused tests cover command isolation, callback filtering, mailbox
handover, and the shared-entry lifetime. The first-party tray template adds an
isolated generated-child pipe test; its [desktop helper](../start-tray-template.bat)
is ready for the remaining real notification-area proof. Right-click Anodrel's
visible icon, choose the enabled command, and verify the child reads the
matching `tray.action.invoked` event before it closes. No synthetic input test
is presented as that user interaction proof.

## Security and privacy

- The host, not the application, owns the notification-area entry, icon,
  tooltip, native command IDs, popup placement, and callback routing.
- Applications receive no raw click, button, keyboard, focus, window,
  visibility, dismissal, timing, or desktop-state data.
- A left click is a host-only foreground request; Windows may decline it.
- The host uses the current session revision and enabled state to reject stale
  or disabled native commands before the action crosses the protocol.
- Sharing the entry with notifications prevents duplicate icon lifetimes and
  makes cleanup run in one place.

## Deferred

Submenus, separators, checkbox or radio state, custom artwork, dynamic
tooltips, badges, close-to-tray behavior, background services, a global
shortcut, window state or focus readback, tray notifications, and non-Windows
adapters all need separate contracts and decisions.

See [notification-area foundation](NOTIFICATION_AREA.md), [native menus](MENUS.md),
[context menus](CONTEXT_MENUS.md), and Decision 0191.
