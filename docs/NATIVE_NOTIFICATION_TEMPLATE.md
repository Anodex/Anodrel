# Anodrel development native notification template

**Status:** The typed Rust facade, explicit generator, fixed Windows host
route, and generated-child private-session test are implemented. A person
seeing the Shell32 notification remains the desktop acceptance check.

## Purpose

This template creates a small first-party Rust executable that submits one
strict Anodrel document, asks the host to hand one fixed title and body to the
Windows notification surface, waits five seconds, then closes only its own
session. Bootstrap records, protocol frames, native callbacks, icons, hover
text, application identity, and host lifecycle code never appear in generated
source.

It is separate from regular, menu-bar, context-menu, tray, form, and
window-control templates. None gains `notification.show` because this template
exists.

## Generator and typed client

Run:

~~~text
anodrel-native-app-tool init-notification <destination> <project-slug> <display-label>
~~~

The command accepts one new directory, a Cargo-compatible slug, and bounded
display text. It writes only `Cargo.toml`, `README.md`, and `src/main.rs`; all
Anodrel dependency paths are checkout-relative. It cannot install, package,
sign, register, trust, select notification text, select a duration, select an
icon, choose an Application User Model ID, add a callback, or select a native
notification-area resource.

The generated app uses only these `anodrel-windows-ui-sdk` methods:

| Method | Input | Protocol | Required grant |
| --- | --- | --- | --- |
| `replace_document_v1` | one exact v1 document string | 1.3 | `ui.document.write` |
| `show_notification` | one fixed title and body | 1.13 | `notification.show` |
| `close` | none | 1.3 | `session.close` |

The facade validates the title and body before sending them. An accepted result
means only that the host handed the values to its operating-system adapter; it
does not mean a notification appeared, was seen, remained visible, or was
acted on.

## Development host route

Run a generated executable through:

~~~text
anodrel-windows-host --native-notification-template-client <client.exe>
~~~

The host creates one fresh session and grants exactly `ui.document.write`,
`notification.show`, and `session.close`. It alone chooses the window,
notification-area entry, icon, hover text, callback message, application
attribution, process handle, pipe worker, and cleanup.

## Quick desktop check

On Windows, run [start-notification-template.bat](../start-notification-template.bat).
It builds a disposable generated app and opens it through the fixed development
route. Look for an **Anodrel native notification** while the **Anodrel Native
Notification Template** window remains open. It closes five seconds after the
host accepts its fixed request and reports the retained temporary project.

The helper makes no product identity, certificate, package, installer, or
machine-policy change. Windows may attribute the notification to
`anodrel-windows-host.exe` until production packaging supplies an Application
User Model ID.

## Verification and limits

The generator integration test builds the generated app, gives it one private
invitation, accepts its document, receives one fixed notification request
through the mailbox, completes that request, and verifies a clean self-close.
It cannot prove Shell32 displayed anything; that is the manual desktop check.

There is no event reader, identifier, replacement, action, callback, toast,
icon, sound, scheduling, grouping, history, configurable text, configurable
duration, background lifetime, network access, product identity, packaging, or
non-Windows host in this template. See Decision 0193 and
[notifications](NOTIFICATIONS.md).
