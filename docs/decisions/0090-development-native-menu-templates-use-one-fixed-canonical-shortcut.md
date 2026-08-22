# Decision 0090: Development native menu templates use one fixed canonical shortcut

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0083 introduced a constrained first-party Rust menu template before
the menu protocol had a keyboard route. Protocol 1.24 now adds the only
accepted shortcut forms, and Decision 0089 keeps them local, semantic, and on
the existing revision-checked delivery path.

The generated menu project is the best no-Node, no-webview proof of that route.
Letting a generator caller choose a shortcut, capability, native key code, or
window setting would weaken the intentionally fixed development boundary.

## Decision

`anodrel-native-app-tool init-menu` will continue to accept only its validated
destination, project slug, and display label. Its generated fixed menu model
will add `Ctrl+Shift+M` to its sole enabled `template.menu.complete` command.
The typed `anodrel-ui-client` menu facade will locally validate the Protocol
1.24 optional shortcut field before sending its existing `menu.replace`
operation at minor version 24.

The generated executable, generator command, and fixed Windows host route gain
no capability, protocol operation, native handle, key-state read, callback, or
application-controlled setting. The host still derives the shortcut display,
only accepts it in the current session window, and delivers its existing typed
menu action through `ui.events.read`.

## Consequences

Positive:

- the direct Windows shortcut route has a first-party compiled test surface
  with no Node.js or webview dependency;
- generator callers cannot turn shortcut choice into an ambient keyboard or
  host-policy configuration surface; and
- the existing generated-child integration test can prove the 1.24 model
  crossed the authenticated pipe and still completed only after the semantic
  event.

Tradeoffs:

- the template demonstrates one fixed shortcut, not a configurable menu
  authoring experience; and
- manual Windows verification still needs a person to test both a menu click
  and `Ctrl+Shift+M` on the visible window.

## Revisit conditions

Revisit before accepting a caller-selected shortcut, another menu item,
capability list, key code, global registration, callback, window setting,
production executable identity, package, or non-Windows template.
