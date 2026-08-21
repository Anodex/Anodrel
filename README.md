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

The registered Windows-session boundary now turns one machine-validated
application record into a fixed service bundle before pipe authentication. Its
identity binds state storage and Credential Manager, while its host-owned
clipboard and HTTPS handoff services remain independent narrow seams. Record
version 1.2 adds the existing storage, credential, and file-operation grants;
UI-bound file services remain unavailable in its non-interactive path.

Registered interactive sessions now also group the authenticated document,
input, close, dialog, and retained-file resources that one host-owned native
application window will consume. This is a verified-launch building block, not
a public window API or a substitute for signed application provisioning.

Protocol 1.16 adds the second narrow public window capability:
an authenticated session carrying the separate `window.state` grant can ask the
host UI thread to minimise, maximise, or restore the one native window it
already owns. It has no target, handle, geometry, focus, state readback, or
event surface; see `docs/WINDOW_STATE.md`.

The Windows pipe also has a host-only stop signal, so lifecycle shutdown can
cancel a pending accept or read without exposing IPC control to applications.
The verified Windows product-session adapter joins that pipe, one locked
signed child launch, and one grouped native UI session under the host's single
lifetime owner.

That path now runs. A development-only signed, machine-provisioned fixture — a
first-party child, a controlled provisioning helper, and a Windows-tooling-only
script — exercises machine policy, locked digest revalidation, Authenticode
publisher match, child-only bootstrap delivery, the authenticated pipe, a
host-owned native window, one semantic action, and coordinated shutdown. The
host activates it through a `--product-session` route, and the Startup Lab's
**Development Fixture** tile is resolved from a verification-only preflight
instead of a compile-time constant, so it is inert unless a machine record and
signed executable currently validate.

This is a development-machine fixture and is never presented as a product
launch. It relies on a locally generated certificate installed into machine
trust, and **that has not been done on any machine yet, so the joined signed
path remains unvalidated in practice**. Production signing identity, packaging,
installation, and updates are deliberately deferred; see `ROADMAP.md` and
`docs/PRODUCT_FIXTURE.md`.

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
start a process. The Startup Lab exposes its launch tile only while a
verification-only preflight confirms that a machine record and signed executable
validate right now.

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
`docs/CRASH_REPORTS.md` defines the host-only bounded record of a contained
panic. It is written to the host's own location, carries no panic payload, and
is readable through no protocol operation at all. It covers contained Rust
panics only, and says so.
`docs/SIGNING.md` defines the Windows executable-signature foundation.
`docs/LAUNCH.md` defines the installed application-record contract and the
host-only Windows launch sequence that binds executable and publisher policy
before process launch.
`docs/PRODUCT_SESSIONS.md` defines the host-only verified Windows product
session and its shutdown rules.
`docs/PRODUCT_FIXTURE.md` defines the development-only signed fixture,
provisioning contract, and host activation routes that exercise it.
`docs/PATHS.md` defines the host-owned per-application directory layout.
`docs/CREDENTIALS.md` defines the host-only Windows credential-store boundary.
`docs/CLIPBOARD.md` defines the bounded text-only clipboard foundation.
`docs/EXTERNAL_LINKS.md` defines the validated HTTPS external-link foundation.
`docs/FILE_DIALOGS.md` defines the bounded file-dialog value foundation.
`docs/FILE_ACCESS.md` defines the planned selection-scoped file-access
boundary.
`docs/NOTIFICATIONS.md` defines the one-way bounded notification boundary,
implemented from portable values through the Shell32 adapter, Protocol 1.13, and
a development diagnostic. It reports only that the host accepted a notification,
never that anyone saw it.
`docs/STORAGE.md` defines the bounded application-state storage boundary and
its recovery and capability contract.
`docs/STARTUP_LAB.md` defines the Windows visual startup-test surface.
`docs/INSTANCE_LIFECYCLE.md` defines the first Windows primary-instance
contract.
`docs/WINDOW_LIFECYCLE.md` defines the multi-window host foundation.
`docs/WINDOW_TITLE.md` defines the first public window capability: an
authenticated session proposes its own window's title and the host composes the
displayed caption with an application-name suffix the proposal cannot suppress
or forge. There is no window target, no read, and no other window property.
`docs/WINDOW_STATE.md` defines the separately granted minimise, maximise, and
restore command for that same session-owned window. It is write-only and cannot
name or inspect any window.
`docs/PERFORMANCE.md` defines how Electron comparisons will be measured.
`docs/UI.md` defines the first owned native UI layout and input foundation.
`docs/APPEARANCE.md` defines the direct Windows high-contrast appearance
adapter used by the native interactive UI labs.
`docs/ACCESSIBILITY.md` defines the Windows accessibility boundary. **UI
Automation reading is implemented and verified:** Narrator reads an Anodrel
surface aloud on Windows 11 and an Inspect cross-check of every published
property passes. An enabled authenticated-session button now exposes one bounded
Invoke action that joins the existing revision-bound semantic-event mailbox;
the provider also reports and can move the host's keyboard-focus snapshot
through a bounded UI-thread route, raises one host-only focus-change event for
a real focus move, and exposes read-only current field values. Manual
screen-reader activation, focus control and event, and field-value verification
remain open. Automation editing, text ranges, Invoke/property/value/text/
structure/selection events, live announcements, and every other pattern remain
absent. See `docs/UI_AUTOMATION_FOCUS.md` and
`docs/UI_AUTOMATION_EVENTS.md`.
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
