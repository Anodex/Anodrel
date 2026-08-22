# Decision 0089: Session menu shortcuts are local, canonical, and semantic

**Status:** Accepted

**Date:** 2026-08-21

## Context

The bounded session menu already lets an application describe semantic commands
without receiving a native menu handle, numeric command ID, callback, or
window target. Common desktop commands also need a keyboard route. Letting an
application register an arbitrary hotkey, name a Windows virtual-key code,
install an accelerator table, intercept text input, or receive raw keyboard
state would broaden that narrow boundary into desktop control or input
observation.

The existing menu activation path already retains the only useful authority:
the host can offer a current menu revision and action identity to the shared
bounded interaction mailbox, and `ui.events.read` revalidates it before an
application receives an event.

## Decision

Protocol 1.24 extends a `menu.replace` item with one optional canonical
`shortcut` string. The only accepted spellings are `Ctrl+<key>` and
`Ctrl+Shift+<key>`, where `<key>` is one uppercase ASCII letter `A` through
`Z` or digit `0` through `9`. A shortcut is unique across the complete menu,
including disabled items. It is a semantic declaration, not a Windows key
code, accelerator-table entry, mnemonic, or callback.

The Windows host displays that host-derived spelling beside the semantic label,
but owns the native behaviour. For an active Anodrel session window, its
direct User32 message handler may accept only the first `WM_KEYDOWN` for a
current enabled declared shortcut while Control is down, Shift exactly matches
the declaration, and Alt is not down. It does not register a global hotkey,
handle `WM_SYSKEYDOWN`, or expose a native keyboard value. Repeats, disabled
items, unmatched keys, and every other window leave normal processing intact.

When it accepts a shortcut, the host creates the exact same
revision-bound `MenuInputCandidate` as a current normal-menu `WM_COMMAND`.
The existing fixed interaction mailbox and granted `ui.events.read` route
remain the only delivery mechanism. There is no keyboard-event payload,
shortcut readback, listener, focus operation, input injection, target, timing,
or evidence that a person used a shortcut.

## Consequences

Positive:

- a common local command route is available without a browser runtime or a
  native handle bridge;
- shortcut and menu-click activation share the same ordered, bounded,
  revision-checked event path; and
- the portable contract remains independent of Windows virtual-key and
  accelerator-table details.

Tradeoffs:

- this deliberately supports a small English canonical subset, not arbitrary
  keyboard layouts, function keys, chords, mnemonics, or localization rules;
- a held key invokes at most once until it is released and pressed again; and
- a shortcut is limited to Anodrel's active session window rather than
  remaining available elsewhere on the desktop.

## Revisit conditions

Revisit before adding any alternative modifier, function or punctuation key,
chord or sequence, platform-global registration, `Alt` mnemonic, OS-specific
key code, accelerator table, input event, key state readback, shortcut
preference, callback, native effect, or non-Windows adapter.
