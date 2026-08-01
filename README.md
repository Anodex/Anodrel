# Anodrel

Anodrel is a reusable native application platform.
It will provide the runtime, security boundaries, platform services, and
communication protocol that multiple desktop applications can use.

Anodex will be the first application built on Anodrel. Anodex is not being
moved into this repository yet; it remains an independent project until the
platform has a stable contract and a working host.

## Status

**Phase:** Foundation implementation

The first implementation slice defines the transport-neutral protocol, client
SDK, mock host, sample application, shared contract tests, a bounded native
transport engine, an authenticated direct Windows named-pipe adapter, and a
direct Windows host. The native layer also has a bounded, private
host-to-child bootstrap adapter for delivering a named-pipe invitation without
command-line or environment-variable secrets. The Windows host proves the
native window lifecycle and protocol core without a runtime framework or a
webview. It also loads a first digest-verified plain-text application
package into a host-controlled Win32 surface. A development-only Node sample
separately exercises the private pipe path end to end; its host-issued grants
include bounded text clipboard, validated HTTPS handoff, and UI-thread-routed
open/save file pickers. Its Protocol 1.9 diagnostic path can also retain one
UI-selected Windows file object and consume its bounded text once through a
separate grant, alongside UI-session test grants. The development client can
also deliver a version 2 scroll tree and complete an action only after it is
revealed by host-retained native scrolling. The first branded Startup Lab turns
those foundation checks, including a temporary private IPC health loopback,
into a direct native visual smoke test.

The authenticated protocol also exposes a bounded read of the host's closed
diagnostic catalogue through its existing diagnostics grant; it accepts no
application log text, native error, filter, or export request. The
application-state foundation now has a portable whole-snapshot contract,
a direct Windows adapter that keeps one recovered prior snapshot, and a
Protocol 1.10 capability surface. The development UI-session diagnostic
exercises its read, replace, and clear path end to end. Installed-application
policy remains a separate integration gate.

Protocol 1.12 now exposes separately granted exact credential read, write, and
delete operations through an injected service bound to the host-validated
application identity. A development-only Windows UI-session diagnostic proves
the current-user Credential Manager path while writing, reading, and removing
one fixed test value; it is not an installed product session and never renders
or logs the credential.

First-party surfaces are drawn by a software renderer rather than by
platform drawing primitives: a portable rasterizer with antialiasing, gradients,
blur, bevels, and filtered image scaling. The Anodrel mark ships as the authored
artwork, committed pre-decoded so the platform displays its real logo while
still shipping no image decoder. Neither crate has an operating-system or
third-party dependency, and both forbid unsafe code, so rendering is tested by
asserting on pixels without opening a window.

The native workspace also includes a first-party performance lab for the owned
wire, authenticated transport, core, and optional local Windows pipe path. It
reports repeatable release measurements without making a comparison claim or
bringing in a benchmark framework. See `docs/PERFORMANCE.md`. Its portable UI
foundation now carries bounded semantic appearance roles, so host renderers do
not rely on element names to choose visual hierarchy. See `docs/UI.md`.

## Goals

- Create a reusable native desktop runtime.
- Keep application logic independent from Electron and any single UI toolkit.
- Provide explicit, versioned interfaces for windows, storage, permissions,
  notifications, dialogs, processes, and other platform services.
- Make security boundaries understandable, testable, and documented.
- Let future applications reuse the platform without copying Anodex internals.
- Keep Anodex and Anodrel in separate repositories with separate Git history.

## Non-goals

- Rebuilding an operating system.
- Rebuilding a browser engine.
- Rewriting all of Anodex in one large migration.
- Avoiding every external development tool; the restriction applies to shipped
  runtime dependencies, not compilers and test tooling.
- Designing a platform around Anodex-only concepts.

## Planned architecture

~~~text
Application
    │
    │ versioned protocol / SDK
    ▼
Platform Core
    │
    │ platform service interfaces
    ▼
Native Host
    │
    ├── Windows
    ├── macOS
    └── Linux
~~~

The current Windows host uses direct Anodrel modules over User32,
Kernel32, and GDI APIs. The direct pipe adapter is restricted to the current Windows
logon session and requires host-created credentials; a separate direct launcher
delivers those credentials once through a child-only anonymous standard-input
handle. Existing TypeScript and React applications remain UI clients through
the SDK rather than importing native APIs. The first host-validated application
package is a deliberately limited text surface. A host-only registered launch
service separately binds executable digest and publisher policy before it can
start a process; the Startup Lab does not expose it until a signed application
is provisioned.

## Repository map

~~~text
Anodrel/
├── apps/                 # Applications built on the platform
├── packages/             # Cross-platform TypeScript packages and SDKs
├── native/               # Native host and platform adapters
├── docs/                 # Architecture, guides, and decision records
├── scripts/              # Development and release helpers
├── tests/                # Cross-component and integration tests
├── AGENTS.md             # Instructions for coding agents
├── ROADMAP.md            # Sequenced project work
└── README.md             # Project entry point
~~~

## Documentation

- AGENTS.md — rules for making changes safely.
- ROADMAP.md — current phases and acceptance gates.
- docs/ARCHITECTURE.md — system boundaries and data flow.
- docs/DEVELOPMENT.md — local workflow and verification.
- docs/decisions/ — durable decisions and their reasoning.

`docs/TRANSPORT.md` defines the native frame and session contract.
`docs/APPLICATIONS.md` defines the validated application-package contract.
`docs/RENDERER.md` documents the renderer and brand API.
`docs/LOGGING.md` defines the bounded host diagnostic-log boundary.
`docs/SIGNING.md` defines the Windows executable-signature foundation.
`docs/LAUNCH.md` defines the installed application-record contract and the
host-only Windows launch sequence that binds executable and publisher policy
before process launch.
`docs/PATHS.md` defines the host-owned per-application directory layout.
`docs/CREDENTIALS.md` defines the host-only Windows credential-store boundary.
`docs/CLIPBOARD.md` defines the bounded text-only clipboard foundation.
`docs/EXTERNAL_LINKS.md` defines the validated HTTPS external-link foundation.
`docs/FILE_DIALOGS.md` defines the bounded file-dialog value foundation.
`docs/FILE_ACCESS.md` defines the planned selection-scoped file-access
boundary.
`docs/STORAGE.md` defines the bounded application-state storage boundary and
its recovery and capability contract.
`docs/STARTUP_LAB.md` defines the Windows visual startup-test surface.
`docs/INSTANCE_LIFECYCLE.md` defines the first Windows primary-instance
contract.
`docs/WINDOW_LIFECYCLE.md` defines the multi-window host foundation.
`docs/PERFORMANCE.md` defines how Electron comparisons will be measured.
`docs/UI.md` defines the first owned native UI layout and input foundation.
`docs/APPEARANCE.md` defines the direct Windows high-contrast appearance
adapter used by the native interactive UI labs.
`docs/UI_DOCUMENTS.md` defines its exact, capability-free external document
format.
`docs/SCROLLING.md` defines the owned scroll-container boundary and its Windows
development diagnostic.
`docs/UI_SESSIONS.md` defines bounded revision and semantic-event state used by
the first capability-checked authenticated UI document replacement path.
`docs/UI_PREVIEW.md` defines the bounded Windows developer preview command.
`docs/UI_SESSION_LAB.md` defines the authenticated native UI delivery smoke
test.

The repository's GitHub Pages landing page lives in `docs/index.html` and uses
only hand-authored HTML and CSS.

## Working rule

Every substantial architectural change must update the relevant documentation
in the same change. The code, tests, and documentation should describe the same
system at all times.

## Current foundation commands

After installing the workspace dependencies, run:

~~~text
npm run check
npm test
npm run demo
~~~

See docs/DEVELOPMENT.md for prerequisites and the expected verification order.

On Windows, double-click `start.bat` from the repository root to build and open
the Anodrel Startup Lab. It validates the sample package and host core, then
completes a temporary private IPC health loopback before the native visual test
surface appears. It builds in release: the surface composes every frame in
software, and an unoptimised build cannot hold its frame rate.

The public interface and security baseline are documented in docs/PROTOCOL.md
and docs/THREAT_MODEL.md.
