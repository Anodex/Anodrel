# Anodrel development native menu template

**Status:** The portable typed menu facade, constrained generator, fixed
Windows host route, and real authenticated generated-child session test are
implemented and verified. The documented manual activation checks—a menu click
and its fixed local shortcut—remain pending. This is a Windows development
template, not a product packaging or application-identity format.

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
accepts the same validated destination, Cargo-compatible slug, and bounded
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

The generated source contains one fixed v1 document and one fixed v1 menu
model. Its menu has a single enabled semantic command,
`template.menu.complete`, with the fixed `Ctrl+Shift+M` local shortcut. It
will not read a document or menu from an argument, file, environment variable,
URL, network connection, or native resource.

## Typed client contract

`anodrel-windows-ui-sdk` exposes these existing closed methods:

| Method | Input | Typed result | Protocol | Required grant |
| --- | --- | --- | --- | --- |
| `replace_menu_v1` | one strict complete menu-model JSON string | `MenuRevision` | 1.24 | `menu.write` |
| `read_events` | none | `UiEventBatch` | 1.24 | `ui.events.read` |

`MenuRevision` parses only a canonical nonzero decimal string. `UiEventBatch`
contains no more than 32 events and the existing bounded `dropped` and
`discarded` counters. Its event variants will be:

- `DocumentAction(UiAction)`, the existing revision-bound document action; or
- `MenuAction(UiMenuAction)`, carrying only a menu revision and semantic action
  ID.

`read_actions` remains for the regular template's document-only behavior. It
rejects a menu event rather than omit or convert it. No method names an
arbitrary protocol operation, chooses an endpoint or capability, observes a
native menu, creates a window, installs a callback, or starts another process.

The menu model and event shape are already fixed by `docs/MENUS.md`. The client
uses the existing 16 KiB model limit, exact object fields, bounded labels,
semantic action grammar, and the Protocol 1.24 optional canonical shortcut
field. The generator exposes no way to alter or add that fixed shortcut.

## Development host session

The implemented command is:

~~~text
anodrel-windows-host --native-menu-template-client <client.exe>
~~~

It creates one host-controlled Windows session and grants exactly:

- `ui.document.write`;
- `ui.events.read`;
- `menu.write`; and
- `session.close`.

The host chooses its own application ID, session ID, window caption,
mailboxes, native menu objects, numeric command mapping, process handle, pipe
worker, and shutdown. The application will not name or inspect a window,
native menu, command identifier, keyboard state, activation source, or another
session.

## Quick desktop check

On Windows, double-click `start-menu-template.bat` in the repository root. It
creates a uniquely named temporary project, builds it from the local checkout,
and opens the generated executable through the fixed menu-template route. In
the **Anodrel Native Menu Template** window, choose **File > Complete menu
template session** once, then run it again and press **Ctrl+Shift+M** once. A
successful close prints a completion message and leaves the disposable source
project in the temporary directory it reports.

The helper creates no certificate, package, installer, application record, or
machine policy. It is a convenience for this one development check; it does not
run a generated executable directly or turn it into a product application.

## Compatibility and verification

This work adds no new wire frame or core protocol operation. `menu.replace`
and `menu.action.invoked` are existing Protocol 1.18 values; the generated
model uses the Protocol 1.24 shortcut field. Typed-facade unit tests now prove
local menu-model validation, request versioning, revision parsing, menu-event
parsing, and document-only failure when a menu event arrives. The generator's
real-pipe integration test builds a generated executable, delivers an
invitation, accepts its fixed shortcut-bearing menu through the host mailbox,
supplies only its fixed semantic menu action, then verifies self-close and
clean exit. The Windows host's fixed-grant lifecycle also has a unit test.
Remaining proof is the documented manual click and **Ctrl+Shift+M** activation
of **Complete menu template session** on a Windows desktop.

No implementation may claim manual menu verification until both activation
paths are observed on a Windows desktop. See Decisions 0083 and 0090.
