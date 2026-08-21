# Decision 0080: Native session menus use bounded semantic commands

**Status:** Accepted

**Date:** 2026-08-21

## Context

Anodrel has bounded document actions, but a desktop application also needs a
familiar top-level command surface. Letting an application post `WM_COMMAND`,
provide a menu handle, edit a system menu, install a callback, or attach an
accelerator would turn that surface into ambient window or process authority.
A separate event callback would also bypass the revision checks and fixed queue
that protect document actions.

Windows already reports a normal menu command through `WM_COMMAND` with high
word zero and `lParam` zero. The low word is a numeric identifier, but that is
a native implementation detail, not an application protocol value.

## Decision

Protocol 1.18 adds one whole-model `menu.replace` operation behind a separate
`menu.write` grant. The first model contains only a bounded set of top-level
labels and enabled semantic command items. The host assigns and retains every
native menu identifier; applications can provide only a validated semantic ID
and display label.

The UI thread is the sole owner of the Windows menu objects and their current
identifier mapping. It accepts a candidate only from the exact normal-menu
`WM_COMMAND` shape and only when the identifier is present in that current
mapping. It offers a revision-bound semantic candidate to the same ordered,
bounded session interaction mailbox that document actions use.

`ui.events.read` remains the only application delivery operation. The core
revalidates the menu revision and enabled command before returning one
`menu.action.invoked` event. There is no callback, native command, operation
name, payload, target, shortcut, menu-state read, or proof that a person used a
menu item.

## Consequences

Positive:

- applications gain a first-party desktop menu bar without a browser runtime
  or a native handle bridge;
- menu and document activation retain one bounded ordering and one
  authenticated pull-delivery path; and
- command labels cannot select system behavior, a window, or a native command.

Tradeoffs:

- the first slice intentionally omits submenus, separators, check state,
  accelerators, context menus, and dynamic command state; and
- a candidate produced while a newer model is being applied may be discarded
  rather than delivered under an ambiguous model revision.

## Revisit conditions

Revisit before adding any submenu, separator, shortcut, check or radio state,
role item, icon, context or system menu, opening callback, command payload,
state readback, native effect, persistent configuration, or another operating
system adapter.
