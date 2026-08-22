# Native session menus

**Status:** Portable model, Protocol 1.18 core, installed-record grant, SDK,
mock host, contract tests, shared ordered interaction delivery, and the
one-request `MenuMailbox` are implemented. The direct Windows UI-thread
attachment, native adapter, and development diagnostic are implemented; manual
verification is pending. The preview typed native client, constrained compiled
menu-project generator, fixed Windows development host route, and real
generated-child session test now also use this boundary without Node.js or an
arbitrary capability selector. Protocol 1.24's bounded local-shortcut contract
is implemented below; its manual Windows verification remains pending.

Anodrel's first menu surface will be a host-owned Windows menu bar for one
authenticated application session. It is a bounded way for an application to
present semantic commands to a person. It is not a command shell, window-menu
editor, shortcut registrar, or native handle bridge.

## Public boundary

Protocol 1.18 adds `menu.replace`. It requires a separate host-issued
`menu.write` capability and accepts exactly one complete model. Protocol 1.24
optionally adds one canonical local shortcut to an item:

~~~json
{
  "menus": [
    {
      "label": "File",
      "items": [
        {
          "id": "document.new",
          "label": "New document",
          "enabled": true,
          "shortcut": "Ctrl+Shift+N"
        }
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
fields fail with `request.payload_invalid`. A 1.18 through 1.23 request has
exactly `id`, `label`, and `enabled` on every item. A 1.24 request may include
one additional `shortcut` string. The encoded request is bounded by the normal
64 KiB wire frame and a stricter 16 KiB menu payload limit.

### Local keyboard shortcuts

In Protocol 1.24, `shortcut` is either absent or exactly `Ctrl+<key>` or
`Ctrl+Shift+<key>`. `<key>` is one uppercase ASCII letter `A` through `Z` or
digit `0` through `9`; `Ctrl+S`, `Ctrl+Shift+M`, and `Ctrl+1` are valid.
Lowercase spelling, whitespace, alternative modifier order, `Alt`, `Win`,
punctuation, function keys, and multi-key chords are invalid. A complete model
cannot repeat a shortcut, even on a disabled item.

The host may display its canonical shortcut spelling beside the item label.
The application still never supplies a native key code or accelerator-table
entry. On Windows, a shortcut is considered only while the same session window
is active, from its first ordinary `WM_KEYDOWN`, while Control is down, Shift
exactly matches the declaration, and Alt is not down. It is not a desktop-wide
hotkey: the host never calls `RegisterHotKey`, accepts an `Alt` system-key
message, or exposes keyboard state. A held key, disabled item, stale model, or
unmatched key is not consumed and has no event.

The successful result is `{ "revision": "1" }`, where `revision` is the
host-owned nonzero decimal revision of the complete current menu. It is not a
window identifier, native menu ID, path, handle, capability, or persistent
permission. A host that has no attached session menu returns only
`menu.unavailable`.

An application supplies no native command number, native accelerator,
mnemonic, separator, icon, check state, submenu, system-menu item, window
target, callback, data payload, or executable action. `shortcut`, when the
request uses Protocol 1.24, is only the limited semantic declaration above.
Menu labels are display text only. The Windows adapter escapes mnemonic markers
before it passes them to User32, so an application cannot claim an `Alt`
shortcut or collide with a host mnemonic. Role items, dynamic menus, checkable
commands, context menus, submenus, and every shortcut extension require
separate contracts.

## Activation delivery

A menu item does not directly invoke application code or a native operation.
When a person chooses a current enabled command from the menu bar or through a
current accepted local shortcut, the host offers one bounded, revision-bound
candidate to the same ordered per-session interaction mailbox used by
authenticated document actions. Protocol `ui.events.read` remains the only
application delivery route and still requires its existing `ui.events.read`
grant. Protocol 1.18 implements this event shape, and the direct Windows
adapter produces it from the host-owned menu bar:

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
never sends an identifier across the protocol. A menu-click activation is
accepted only from a `WM_COMMAND` message whose high word is zero and whose
`lParam` is zero—the documented shape of a menu notification—and whose
low-word identifier belongs to the current host mapping. Protocol 1.24's direct
local key route does not broaden that filter: it produces the same candidate
only under its documented `WM_KEYDOWN` and modifier conditions. Every other
`WM_COMMAND` and keyboard message remains outside this contract.

The UI thread replaces the bar and mapping as one host operation. It preserves
the old complete menu if native construction or attachment fails, and it drops
the previous native resources only after replacement succeeds. The pipe worker
never calls User32 or receives a native menu handle. The application cannot
add to the system menu, target another window, or observe menu focus, opening,
selection, dismissal, shortcut handling, or whether a person saw a command.

## Compatibility and deferred work

`menu.replace` is additive in Protocol 1.18. Protocol 1.24's optional
`shortcut` field is rejected by older request versions, preserving their exact
item grammar. Earlier clients and installed records cannot name `menu.write`;
record version 1.8 adds that optional grant as a strict superset of version
1.7. No extra capability, record version, or event shape is required for a
shortcut because it configures an existing menu action and travels through its
existing delivery route. The SDK and mock host implement the portable contract,
while a core without an attached native menu service returns `menu.unavailable`.
The direct Windows host must not grant `menu.write` until its UI-thread bridge,
native adapter, activation delivery, and manual verification agree. The
development-only menu diagnostic is the explicit exception: it grants
`menu.write` only to prove that implementation, and its real menu-bar and
shortcut checks remain documented operator steps.

Submenus, separators, check and radio state, global shortcuts, `Alt` mnemonics,
function and punctuation keys, key chords, native window or system-menu edits,
context menus, menu opening callbacks, dynamic enablement, icons, localization
resource lookup, menu state readback, application callbacks, command payloads,
persistent preferences, and non-Windows adapters are intentionally deferred.

See Decisions 0080 and 0089, `docs/PROTOCOL.md`, `docs/UI_SESSIONS.md`, and
`docs/THREAT_MODEL.md`.
