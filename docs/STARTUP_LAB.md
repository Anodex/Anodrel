# Anodrel Startup Lab

**Status:** Windows foundation test surface.

## Purpose

The Startup Lab is Anodrel's first branded native startup screen. It is a
first-party Win32 diagnostic surface, not an Electron clone, web page, or
application renderer. Its dark indigo and cyan visual system communicates the
platform's owned, bounded, security-first direction while giving developers a
fast visual smoke test.

The lab uses only Anodrel-owned Rust and direct User32/GDI APIs. It contains no
webview, browser engine, JavaScript, external resource fetch, link, navigation,
script execution, or native bridge.

## Launch contract

The Windows host accepts:

~~~text
anodrel-windows-host --showcase <anodrel.application.json>
~~~

Before creating a window, this route must:

1. validate the supplied application package through anodrel-application;
2. perform the host's internal platform.health protocol-core check; and
3. complete one temporary, owned named-pipe loopback: authenticate the first
   `ANDR` frame and make one `platform.health` request; and
4. fail closed with a safe error if any check fails.

start.bat launches the sample manifest through this route. The screen therefore
verifies the sample package's canonical containment and SHA-256 content digest
in the normal developer startup path.

## What the screen demonstrates

| Lab card | Actual condition | What it does not claim |
| --- | --- | --- |
| Owned Core | The host constructed and processed its internal platform.health request. | A public application session or a privileged capability. |
| Verified Package | The supplied manifest and its bounded text content passed containment and digest checks. | Publisher signing or verified executable identity. |
| Private IPC | A temporary current-session pipe accepted an owned in-process loopback client, which authenticated and completed one `platform.health` round trip. | A public application client, executable launch, bootstrap selection, or any privileged capability. |

The title and identity line come from the validated manifest. Content text,
untrusted request values, credentials, raw paths, and native error details are
never rendered on the Startup Lab surface.

## Visual contract

The screen owns its complete client-area drawing:

- deep midnight background, indigo panels, cyan and violet accents;
- an Anodrel orbital mark made from direct GDI geometry;
- host-owned labels, status cards, and a bounded developer command line;
- one responsive layout that keeps the cards and identity visible at the
  supported window size.

The design deliberately borrows the useful role of Electron's welcome screen:
a first-run orientation and a visual test point. Its mark, palette, wording,
layout, and implementation are Anodrel's own.

## Manual verification

From the repository root, double-click start.bat. Confirm that an
**Anodrel Startup Lab** window opens and shows:

- the org.anodrel.sample identity;
- Owned Core and Verified Package as ready;
- Private IPC as a ready loopback self-test; and
- the Anodrel orbital mark and three status cards on the dark native surface.

Close the window normally. A changed or invalid manifest/content pair must
prevent the Startup Lab window from opening. A failed private IPC handshake or
health round trip must also prevent it from opening.
