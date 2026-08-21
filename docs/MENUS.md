# Native session menus

**Status:** Portable model, Protocol 1.18 core, installed-record grant, SDK,
mock host, contract tests, shared ordered interaction delivery, and the
one-request `MenuMailbox` are implemented. The direct Windows UI-thread
attachment, native adapter, and development diagnostic are implemented; manual
verification is pending. The preview typed native client, constrained compiled
menu-project generator, fixed Windows development host route, and real
generated-child session test now also use this boundary without Node.js or an
arbitrary capability selector. A documented manual Windows menu click remains
the final acceptance check for that development template.

Anodrel's first menu surface will be a host-owned Windows menu bar for one
authenticated application session. It is a bounded way for an application to
present semantic commands to a person. It is not a command shell, window-menu
editor, shortcut registrar, or native handle bridge.

## Public boundary

Protocol 1.18 adds `menu.replace`. It requires a separate host-issued
`menu.write` capability and accepts exactly one complete model:

~~~json
{
  "menus": [
    {
      "label": "File",
      "items": [
        { "id": "document.new", "label": "New document", "enabled": true }
      ]
    }
  ]
}
~~~

The model has one to eight top-level menus. Each has a one-to-32-byte
non-control UTF-8 label and one to sixteen items. An item has one unique
existing `ElementId`-shaped `id`, a one-to-96-byte non-control UTF-8 label, and
an explicit boolean `enabled` value. At most 128 command items exist in one
model. Every object is exact: unknown, missing, duplicate, or wrongly typed
fields fail with `request.payload_invalid`. The encoded request is bounded by
the normal 64 KiB wire frame and a stricter 16 KiB menu payload limit.

The successful result is `{ "revision": "1" }`, where `revision` is the
host-owned nonzero decimal revision of the complete current menu. It is not a
window identifier, native menu ID, path, handle, capability, or persistent
permission. A host that has no attached session menu returns only
`menu.unavailable`.

An application supplies no native command number, accelerator, mnemonic,
separator, icon, check state, submenu, system-menu item, window target,
callback, data payload, or executable action. In this first slice menu labels
are display text only. The Windows adapter escapes mnemonic markers before it
passes them to User32, so an application cannot claim an `Alt` shortcut or
collide with a host mnemonic. Keyboard accelerators, role items, dynamic menus,
checkable commands, context menus, and submenus require separate contracts.

## Activation delivery

A menu item does not directly invoke application code or a native operation.
When a person chooses a current enabled command, the host offers one bounded,
revision-bound candidate to the same ordered per-session interaction mailbox
used by authenticated document actions. Protocol `ui.events.read` remains the
only application delivery route and still requires its existing
`ui.events.read` grant. Protocol 1.18 implements this event shape, and the
direct Windows adapter produces it from the host-owned menu bar:

~~~json
{
  "eventName": "menu.action.invoked",
  "payload": { "menuRevision": "1", "action": "document.new" }
}
~~~

Its complete event envelope uses `source: "native.menu"` and
`schemaVersion: { "major": 1, "minor": 18 }`. It has no window identifier,
menu identifier, position, timing, keyboard state, or evidence that a person
saw a command.

Before delivery, the core confirms that the candidate's menu revision is still
current and the matching command remains enabled. A menu replaced, disabled,
or removed after a person opened it is discarded. Document actions and menu
actions share one fixed 32-candidate queue, so their delivery order is
preserved and a menu cannot create an unbounded second event path. The existing
`dropped` and `discarded` counts cover both kinds of semantic interaction.

## UI-thread replacement bridge

The authenticated pipe worker does not hold a window or call User32. It gives
one already validated complete model and its host-owned revision to a
per-session `MenuMailbox`, then waits at most five seconds for the owning UI
thread. The mailbox holds at most one request, transfers it exactly once, and
accepts a completion only for that exact taken request. A second request while
one is pending or being applied, an absent UI thread, a timed-out request, or a
native replacement failure all answer `menu.unavailable`; the core therefore
retains its prior portable menu state.

The UI thread creates the next native menu before replacing the window's
current one. It completes the bridge only after `SetMenu` succeeds, then
destroys the old menu itself. This preserves the last working menu on every
failure path. The mailbox carries no window, handle, native command ID,
callback, or operating-system call.

## Windows ownership

The Windows UI thread alone creates, attaches, replaces, and destroys the
native menu bar. It assigns private numeric command identifiers only after the
portable model validates, retains their mapping in the host session view, and
never sends an identifier across the protocol. A menu activation is accepted
only from a `WM_COMMAND` message whose high word is zero and whose `lParam` is
zero—the documented shape of a menu notification—and whose low-word identifier
belongs to the current host mapping. Every other `WM_COMMAND` remains outside
this contract.

The UI thread replaces the bar and mapping as one host operation. It preserves
the old complete menu if native construction or attachment fails, and it drops
the previous native resources only after replacement succeeds. The pipe worker
never calls User32 or receives a native menu handle. The application cannot
add to the system menu, target another window, or observe menu focus, opening,
selection, dismissal, shortcut handling, or whether a person saw a command.

## Compatibility and deferred work

`menu.replace` is additive in Protocol 1.18. Earlier clients and installed
records cannot name `menu.write`; record version 1.8 adds that optional grant
as a strict superset of version 1.7. The SDK and mock host implement the
portable contract, while a core without an attached native menu service returns
`menu.unavailable`. The direct Windows host must not grant `menu.write` until
its UI-thread bridge, native adapter, activation delivery, and manual
verification agree. The development-only menu diagnostic is the explicit
exception: it grants `menu.write` only to prove that implementation, and its
real menu-bar check remains a documented operator step.

Submenus, separators, check and radio state, keyboard accelerators, native
window or system-menu edits, context menus, menu opening callbacks, dynamic
enablement, icons, localization resource lookup, menu state readback,
application callbacks, command payloads, persistent preferences, and
non-Windows adapters are intentionally deferred.

See Decision 0080, `docs/PROTOCOL.md`, `docs/UI_SESSIONS.md`, and
`docs/THREAT_MODEL.md`.
