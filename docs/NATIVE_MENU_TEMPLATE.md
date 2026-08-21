# Anodrel development native menu template

**Status:** Contract accepted; implementation pending. This is a Windows
development template, not a product packaging or application-identity format.

## Purpose

The menu template will create a first-party Rust executable that submits one
strict Anodrel UI document, replaces one strict native session-menu model,
waits for one semantic menu action, then closes its own session. It keeps
bootstrap records, wire frames, request IDs, raw protocol JSON envelopes, and
host orchestration out of generated application source.

It is separate from the regular native UI template in `docs/NATIVE_UI_TEMPLATE.md`.
The regular template does not gain menu authority merely because this template
exists.

## Generator contract

`anodrel-native-app-tool init-menu <destination> <project-slug> <display-label>`
will accept the same validated destination, Cargo-compatible slug, and bounded
display label as `init`. It will refuse an existing destination and write only:

~~~text
my-native-menu-app/
|- Cargo.toml
|- README.md
`- src/
   `- main.rs
~~~

Every Anodrel dependency path will remain relative to the local checkout. The
tool will not install, run, sign, package, register, or trust the executable.
It will not accept a capability list, menu handle, command number, shortcut,
title, window setting, or menu source path.

The generated source will contain one fixed v1 document and one fixed v1 menu
model. Its menu has a single enabled semantic command,
`template.menu.complete`. It will not read a document or menu from an argument,
file, environment variable, URL, network connection, or native resource.

## Typed client contract

`anodrel-ui-client` will extend its preview surface with:

| Method | Input | Typed result | Protocol | Required grant |
| --- | --- | --- | --- | --- |
| `replace_menu_v1` | one strict complete menu-model JSON string | `MenuRevision` | 1.18 | `menu.write` |
| `read_events` | none | `UiEventBatch` | 1.18 | `ui.events.read` |

`MenuRevision` will parse only a canonical nonzero decimal string. `UiEventBatch`
will contain no more than 32 events and the existing bounded `dropped` and
`discarded` counters. Its event variants will be:

- `DocumentAction(UiAction)`, the existing revision-bound document action; or
- `MenuAction(UiMenuAction)`, carrying only a menu revision and semantic action
  ID.

`read_actions` remains for the regular template's document-only behavior. It
will reject a menu event rather than omit or convert it. No method names an
arbitrary protocol operation, chooses an endpoint or capability, observes a
native menu, creates a window, installs a callback, or starts another process.

The menu model and event shape are already fixed by `docs/MENUS.md`. The client
will use the existing 16 KiB model limit, exact object fields, bounded labels,
and semantic action grammar.

## Development host session

The planned command is:

~~~text
anodrel-windows-host --native-menu-template-client <client.exe>
~~~

It will create one host-controlled Windows session and grant exactly:

- `ui.document.write`;
- `ui.events.read`;
- `menu.write`; and
- `session.close`.

The host will choose its own application ID, session ID, window caption,
mailboxes, native menu objects, numeric command mapping, process handle, pipe
worker, and shutdown. The application will not name or inspect a window,
native menu, command identifier, keyboard state, activation source, or another
session.

## Compatibility and verification

This work adds no new wire frame or core protocol operation. `menu.replace`
and `menu.action.invoked` are existing Protocol 1.18 values. The work will add
typed-facade unit tests for local validation, request versioning, response
parsing, and both event shapes; a generated-project release build; a real
authenticated generated-child menu-session test; host fixed-grant tests; and a
documented manual click of **Complete menu template session** from the Windows
menu bar.

No implementation may claim manual menu verification until that last action is
observed on a Windows desktop. See Decision 0083.
