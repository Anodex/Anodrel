# Decision 0083: Development native menu templates add one fixed menu grant

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0080 provides a bounded native Windows menu bar through Protocol 1.18.
Its portable model, core policy boundary, UI-thread bridge, direct User32
adapter, and development diagnostic already exist. Decision 0082 deliberately
kept the first generated native UI project to three operations: document
replacement, semantic-action read, and session close. That made the first
template useful without presenting a broad capability selector, but it leaves a
compiled first-party application unable to use the existing menu surface.

Adding `menu.write` to every generated project would silently broaden the
default development authority. Letting a project pass a capability list, native
command identifier, keyboard shortcut, menu callback, or host route would make
the generator an unverified launcher rather than a constrained development
tool.

## Decision

Keep `anodrel-native-app-tool init` and `--native-template-client` unchanged.
They continue to grant only `ui.document.write`, `ui.events.read`, and
`session.close`.

Add a separate exact `init-menu` generator command and a separate exact
`--native-menu-template-client <client.exe>` host route. The route creates one
fresh development session under its fixed host application ID and session ID.
It grants exactly:

- `ui.document.write`;
- `ui.events.read`;
- `menu.write`; and
- `session.close`.

The host does not receive any project-supplied capability, application ID,
session ID, window title, native menu handle, command number, shortcut, or
callback. The generated executable is still explicitly selected unverified
development code, not a package, product launcher, installed record, or signed
application.

Extend the preview `anodrel-ui-client` facade with a closed menu surface:

- `replace_menu_v1` accepts one exact bounded Protocol 1.18 menu-model JSON
  value, validates it locally, sends only `menu.replace` at Protocol 1.18, and
  returns a validated nonzero `MenuRevision`;
- `read_events` sends only `ui.events.read` at Protocol 1.18 and returns a
  bounded ordered batch of typed document actions and typed menu actions; and
- the existing `read_actions` stays document-only and fails closed if a menu
  event appears, so an older client cannot silently discard a menu action.

`MenuRevision` is a canonical nonzero decimal value. A menu action carries only
that revision and a validated semantic menu action ID. It has no menu ID,
window ID, native command number, pointer location, shortcut state, or proof
that a person saw the menu. The generated menu project compiles one strict
menu model and completes only after its exact enabled menu action returns
through `ui.events.read`, then requests close of its own session.

## Consequences

- The first-party compiled native path can exercise a real Windows menu bar
  without Node.js, a webview, or raw protocol construction in generated source.
- A caller chooses the capability-bearing template by selecting an explicit
  generator command and host route, not by passing free-form authority to a
  generic tool.
- The normal UI template retains its three-grant least-privilege boundary.
- Existing menu semantics and their one shared, revision-checked interaction
  queue remain the only delivery path; no new callback or event channel exists.

## Revisit conditions

Revisit before adding menu capabilities to the default template, accepting a
capability list, exposing native menu or window identifiers, adding shortcuts,
submenus, context or system menus, command payloads, concurrent event readers,
another operating-system adapter, packaging, or production executable identity.
Each changes either the capability or launch boundary.
