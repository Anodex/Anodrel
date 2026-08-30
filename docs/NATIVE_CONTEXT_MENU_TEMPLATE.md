# Anodrel development native context-menu template

**Status:** The typed Rust facade, explicit generator, fixed Windows host
route, generated-child real-pipe test, and direct User32 popup are implemented.
The remaining acceptance check is one manual right-click on a Windows desktop.
This is a development template, not a package, installer, signing format, or
product application identity.

## Purpose

The template creates a small first-party Rust executable that submits one
strict Anodrel UI document, replaces one strict semantic context-menu model,
waits for one local popup action, and closes only its own session. It keeps
bootstrap records, protocol frames, request IDs, native coordinates, popup
handles, command numbers, and host lifecycle code out of generated source.

It is separate from the regular and menu-bar templates. Neither gains
`menu.context.write` merely because this template exists.

## Generator and typed client

Run:

~~~text
anodrel-native-app-tool init-context-menu <destination> <project-slug> <display-label>
~~~

The command accepts one new directory, a Cargo-compatible slug, and bounded
display text. It writes only `Cargo.toml`, `README.md`, and `src/main.rs`; all
Anodrel dependency paths are checkout-relative. It does not install, package,
sign, register, or trust the executable, and it cannot accept a capability
list, pointer position, target, callback, shortcut, command number, window
handle, or context-menu source file.

The generated app uses only these methods from `anodrel-windows-ui-sdk`:

| Method | Input | Typed result | Protocol | Required grant |
| --- | --- | --- | --- | --- |
| `replace_context_menu_v1` | one exact context-menu JSON string | `ContextMenuRevision` | 1.32 | `menu.context.write` |
| `read_context_menu_actions` | none | `UiContextMenuActionBatch` | 1.32 | `ui.events.read` |
| `replace_document_v1` | one exact v1 document string | `DocumentRevision` | 1.3 | `ui.document.write` |
| `close` | none | accepted close request | 1.3 | `session.close` |

`ContextMenuRevision` accepts only a canonical nonzero decimal string.
`UiContextMenuActionBatch` contains at most 32 actions plus bounded `dropped`
and `discarded` counts. An action carries only its revision and semantic ID.
It never contains a pointer position, trigger target, selection, link, popup
state, native handle, or command number.

## Development host route

Run a generated executable through:

~~~text
anodrel-windows-host --native-context-menu-template-client <client.exe>
~~~

The host creates one fresh session and grants exactly:

- `ui.document.write`;
- `ui.events.read`;
- `menu.context.write`; and
- `session.close`.

The host alone chooses the window, pointer trigger, screen placement, User32
popup lifetime, command mapping, process handle, pipe worker, and cleanup. The
generated program cannot inspect or choose any of those values.

## Quick desktop check

On Windows, run [start-context-menu-template.bat](../start-context-menu-template.bat).
It builds a disposable generated app and opens it through the fixed development
route. In the **Anodrel Native Context Menu Template** window, right-click
inside the client area and choose **Complete context-menu template session**.
The process should close successfully and report the temporary project path.

The helper makes no product identity, certificate, package, installer, or
machine-policy change. It retains the temporary source project for inspection.

## Verification and limits

The typed client validates the exact model before sending it and validates the
response event before returning it. The generator's real-pipe integration test
builds the generated application, gives it one private invitation, accepts its
model through the mailbox, supplies its one semantic candidate, and verifies a
clean self-close. Host tests cover signed multi-monitor coordinates and reject
the keyboard-originated `(-1,-1)` trigger before User32 receives a popup call.

Manual verification must use the real right-click route. Keyboard invocation,
submenus, separators, check/radio state, icons, shortcuts, dynamic enablement,
target nodes, selection/link facts, callbacks, secondaries, non-Windows hosts,
and browser integration remain outside this template and require a new
decision. See Decision 0121 and `docs/CONTEXT_MENUS.md`.
