# Anodrel development native tray template

**Status:** The typed Rust facade, explicit generator, fixed Windows host
route, and generated-child real-pipe test are implemented. A real
notification-area right-click remains the desktop acceptance check.

## Purpose

This template creates a small first-party Rust executable that submits one
strict Anodrel document, replaces one strict semantic tray model, waits for one
notification-area action, and closes only its own session. Bootstrap records,
protocol frames, native callbacks, icons, tooltips, coordinates, popup handles,
command numbers, and host lifecycle code never appear in generated source.

It is separate from regular, menu-bar, and context-menu templates. None gains
`tray.write` merely because this template exists.

## Generator and typed client

Run:

~~~text
anodrel-native-app-tool init-tray <destination> <project-slug> <display-label>
~~~

The command accepts one new directory, a Cargo-compatible slug, and bounded
display text. It writes only `Cargo.toml`, `README.md`, and `src/main.rs`; all
Anodrel dependency paths are checkout-relative. It cannot install, package,
sign, register, trust, or select a capability, icon, tooltip, callback,
coordinate, command number, window handle, or tray source file.

The generated app uses only these `anodrel-windows-ui-sdk` methods:

| Method | Input | Typed result | Protocol | Required grant |
| --- | --- | --- | --- | --- |
| `replace_tray_v1` | one exact tray-model JSON string | `TrayRevision` | 1.33 | `tray.write` |
| `read_tray_actions` | none | `UiTrayActionBatch` | 1.33 | `ui.events.read` |
| `replace_document_v1` | one exact v1 document string | `DocumentRevision` | 1.3 | `ui.document.write` |
| `close` | none | accepted close request | 1.3 | `session.close` |

`TrayRevision` accepts only a canonical nonzero decimal string.
`UiTrayActionBatch` contains at most 32 actions plus bounded `dropped` and
`discarded` counts. An action carries only its revision and semantic ID, never
an icon, tooltip, click, pointer, popup state, native handle, or command number.

## Development host route

Run a generated executable through:

~~~text
anodrel-windows-host --native-tray-template-client <client.exe>
~~~

The host creates one fresh session and grants exactly `ui.document.write`,
`ui.events.read`, `tray.write`, and `session.close`. It alone chooses the
window, notification-area entry, icon, tooltip, callback, popup placement,
User32 command mapping, process handle, pipe worker, and cleanup.

## Quick desktop check

On Windows, run [start-tray-template.bat](../start-tray-template.bat). It
builds a disposable generated app and opens it through the fixed development
route. In the **Anodrel Native Tray Template** window, right-click Anodrel's
notification-area icon and choose **Complete tray template session**. The
process should close successfully and report the retained temporary project.

The helper makes no product identity, certificate, package, installer, or
machine-policy change.

## Verification and limits

The facade validates the complete model before sending it and validates the
event before returning it. The generator integration test builds the generated
app, gives it one private invitation, accepts its model through the mailbox,
supplies one semantic candidate, and verifies a clean self-close.

Manual verification must use the real notification-area right-click. Submenus,
separators, check/radio state, icons, tooltip changes, badges, close-to-tray,
background services, global shortcuts, custom click behavior, callbacks,
coordinates, non-Windows hosts, and browser integration remain outside this
template. See Decision 0192 and [native tray menus](TRAY.md).
