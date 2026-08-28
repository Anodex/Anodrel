# Anodrel

Anodrel is a native-first platform for building secure, modular desktop
applications. It gives applications a shared foundation of direct
operating-system services, explicit permissions, and versioned local IPC—without
shipping a browser runtime.

## What Anodrel is

- A reusable desktop platform, beginning with a direct Windows host.
- A security-focused boundary between application code and operating-system
  services: every capability is explicit, bounded, and documented.
- A modular native foundation built on platform APIs rather than a bundled
  browser engine, webview, or Node.js runtime.

It is not an application itself, an Anodex source mirror, or a finished
Electron replacement. Production packaging, signing, installation, updates,
and macOS/Linux hosts remain planned work.

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
package into a host-controlled Win32 surface. Development-only samples
separately exercise the private pipe path end to end: compiled native probes
check bootstrap, authentication, health, and one complete host-owned UI session
without a runtime dependency, while the Node sample checks the broader service
path. Its host-issued grants include
bounded text clipboard, validated HTTPS handoff, and UI-thread-routed open/save
file pickers. Its Protocol 1.9 diagnostic path can also retain one
UI-selected Windows file object and consume its bounded text once through a
separate grant, alongside UI-session test grants. The development client can
also deliver a version 2 scroll tree and complete an action only after it is
revealed by host-retained native scrolling. The first branded Startup Lab turns
those foundation checks, including a temporary private IPC health loopback,
into a direct native visual smoke test.

Protocol 1.17 also provides a separately granted, selection-scoped text-write
path in the direct Windows UI-session host. A host-owned save picker captures
one native output object under a one-use opaque reference; the application can
write at most 8 KiB of text through that reference and never receives a
reusable filesystem-write API. The legacy save picker remains non-mutating.

Protocol 1.18 now provides the bounded portable foundation for a native
session menu: a separately granted complete semantic model, host-owned
revisions, an installed-record grant, SDK support, and contract coverage. The
direct Windows UI-thread bridge, User32 menu bar, and bounded activation
delivery are also implemented. Protocol 1.24 optionally adds a canonical local
shortcut to the same semantic activation path. A development diagnostic waits
for one real menu click or local shortcut through the authenticated pull path;
its manual verification is the remaining acceptance step. An unattached host
still returns only `menu.unavailable`.

Protocol 1.19 adds one direct, host-authorized HTTPS text-fetch boundary. It
uses strict URLs, an exact host-selected origin policy, a 32 KiB UTF-8 response
limit, and WinHTTP without a browser runtime, proxy discovery, cookies,
redirects, or automatic authentication. A separate compiled Windows diagnostic
tests only the fixed `example.com:443` origin. A validated installed record at
version 1.14 can separately grant `network.fetch` with one to eight exact
machine-selected HTTPS origins; the registered Windows session then receives
the same bounded direct service. Templates and the product fixture still do not
receive it.

Protocol 1.20 adds a deliberately narrow session-window attention request. An
authenticated session carrying the separate `window.focus` grant can ask the
host UI thread to call Windows for its own window only. There is no target,
handle, focus readback, retry, input, or cross-window route; Windows remains
free to decline or flash the taskbar instead. The typed SDK, mock host,
installed-record version 1.9 policy, direct User32 adapter, and development
diagnostic are implemented; its desktop foreground-policy check remains a
manual verification. See `docs/WINDOW_FOCUS.md`.

Protocol 1.21 adds a separately granted reversible session-window fullscreen
request. An authenticated session carrying `window.fullscreen` can select only
borderless `fullscreen` or restored `windowed` presentation for its own host
window. The direct Windows host keeps the original style and placement private,
uses the monitor Windows already associates with that window, and never exposes
a handle, monitor, geometry, display mode, fullscreen state, event, or
cross-window route. The typed SDK, mock host, record version 1.10 policy,
direct User32 and monitor adapter, and development diagnostic are implemented;
its desktop entry-and-restore check remains manual. See
`docs/WINDOW_FULLSCREEN.md`.

Protocol 1.22 adds a separately granted bounded binary-output path to the
direct Windows UI-session host. A `dialog.save_file.v2` selection can be
consumed once by either the existing text writer or `file.write_binary`, whose
first-party canonical base64url boundary accepts at most 32 KiB of decoded
bytes. It exposes no path, handle, MIME type, streaming, readback, or general
filesystem API. Record version 1.11 carries the optional grant; its Windows
picker diagnostic remains a manual desktop verification. See
`docs/FILE_BINARY_WRITE.md`.

Protocol 1.23 adds a separately granted bounded client-area sizing request to
the direct Windows UI-session host. `window.size.set` accepts only whole
96-DPI logical width and height values for the authenticated session's own
window. The host derives its framed native rectangle at the window's current
DPI and preserves position, activation, and z-order; the request exposes no
target, handle, monitor, position, bounds, DPI, or geometry readback. Record
version 1.12 carries the optional grant; its Windows scaling and fullscreen
interaction checks remain manual. See `docs/WINDOW_SIZE.md`.

Protocol 1.24 adds optional bounded local keyboard shortcuts to the existing
native session-menu model. A menu item can declare only `Ctrl+<key>` or
`Ctrl+Shift+<key>` for one uppercase ASCII letter or digit. The direct Windows
host recognizes those declarations only in its own active session window and
delivers the same revision-checked semantic event as a menu click. It exposes
no global hotkey, raw keyboard data, native accelerator, callback, or shortcut
readback. See `docs/MENUS.md`.

Protocol 1.25 adds a bounded session-owned multi-window API. A session with
the separate `window.open` and `window.close` grants can create or request the
close of up to three secondary views, each with its own strict v1 document,
revision, input queue, and opaque session-local identity. Applications cannot
select native geometry, a monitor, parent, handle, or another session's view;
the direct Windows host resolves every identity through its private group map
on the UI thread. `ui.document.replace.window` updates `main` or a known
secondary, while `ui.events.read.window` returns only revision-checked semantic
events tagged by their logical view. Installed record version 1.13 adds the
two new optional grants. See `docs/MULTI_WINDOW.md`.

Protocol 1.26 adds an exact version-3 UI document with one visible semantic
status result. Authenticated primary and secondary session views can use
explicit v3 replacement operations under their existing document-write grant;
the direct Windows host maps a later changed visible status to one best-effort
outbound UI Automation live-region notification. Applications receive no
listener, delivery, focus, or accessibility-presence result. See
`docs/UI_LIVE_ANNOUNCEMENTS.md`.

Protocol 1.27 adds exact v2 opening and replacement operations for scroll-only
secondary session views. Their scroll positions and native input stay local to
each host-owned view; applications receive no position, event, callback, or
native handle. See `docs/SCROLLING.md` and `docs/MULTI_WINDOW.md`.

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
- docs/ARCHITECTURE_FOUNDATIONS.md — cross-cutting architecture rules.
- docs/DEVELOPMENT.md — local workflow and verification.
- docs/CONTINUOUS_INTEGRATION.md — automated repository verification and its
  deliberate limits.
- docs/DEVELOPMENT_DIAGNOSTICS.md — native diagnostics and product-fixture checks.
- docs/PROTOCOL.md — boundary, compatibility, responses, and security rules.
- docs/PROTOCOL_OPERATIONS.md — request-specific payload and operation rules.
- docs/decisions/ — durable decisions and their reasoning.

`docs/TRANSPORT.md` defines the native frame and session contract.
`docs/APPLICATIONS.md` defines the validated application-package contract.
`docs/APPLICATION_TEMPLATE.md` shows how to create and run the first strict,
digest-verified native text package without reaching into host source code.
`docs/SDK.md` defines the public application client and the boundary between a
typed request helper and host-owned native authority.
`docs/WINDOWS_NATIVE_SDK.md` defines the stable in-repository Windows native
application facade over one authenticated invited session.
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
`docs/NETWORK.md` defines the implemented portable host-authorized HTTPS
text-fetch contract, SDK, mock-host boundary, direct WinHTTP adapter, and
fixed-origin Windows development diagnostic. Decision 0099 adds the exact
machine-selected origin policy for version 1.14 installed records.
`docs/FILE_DIALOGS.md` defines the bounded file-dialog value foundation.
`docs/FOLDER_DIALOGS.md` defines the separate bounded folder-selection
contract, its direct Windows Common Item Dialog adapter, and its remaining
manual desktop check.
`docs/FOLDER_ACCESS.md` defines the implemented Protocol 1.29, SDK/mock,
policy, core, and direct Windows route for a separately granted, one-use
retained-folder reference and bounded direct-entry snapshot. Windows enumerates
from the retained directory handle rather than reopening a path; its final
desktop picker check remains a manual verification step.
`docs/FILE_ACCESS.md` defines the implemented selection-scoped file-access
boundary.
`docs/FILE_WRITE.md` defines the separately scoped retained-output-object
text-write boundary implemented in Protocol 1.17 for the direct Windows
UI-session host; legacy save selection remains non-mutating.
`docs/FILE_BINARY_WRITE.md` defines the separately granted bounded binary
output boundary implemented in Protocol 1.22 for that same direct Windows
UI-session host.
`docs/MENUS.md` defines the bounded native session-menu contract and records
the implemented direct Windows adapter, canonical local keyboard shortcuts,
its explicit ownership boundary, and the remaining manual verification.
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
`docs/MULTI_WINDOW.md` defines the bounded session-owned multi-window contract
implemented by Protocol 1.25, including its opaque identities and private
native mapping boundary.
`docs/WINDOW_TITLE.md` defines the first public window capability: an
authenticated session proposes its own window's title and the host composes the
displayed caption with an application-name suffix the proposal cannot suppress
or forge. There is no window target, no read, and no other window property.
`docs/WINDOW_STATE.md` defines the separately granted minimise, maximise, and
restore command for that same session-owned window. It is write-only and cannot
name or inspect any window.
`docs/WINDOW_FOCUS.md` defines the separately granted request for Windows to
foreground that same session-owned window, without exposing focus state or
desktop-control authority.
`docs/WINDOW_FULLSCREEN.md` defines the separately granted reversible
borderless fullscreen request for that same session-owned window, without
exposing monitor selection, display control, geometry, or fullscreen state.
`docs/WINDOW_SIZE.md` defines the separately granted bounded logical
client-area request for that same session-owned window, without exposing
position, outer geometry, monitor, DPI, or readback.
`docs/PERFORMANCE.md` defines how Electron comparisons will be measured.
`docs/UI.md` defines the first owned native UI layout and input foundation.
`docs/APPEARANCE.md` defines the direct Windows high-contrast appearance
adapter used by the native interactive UI labs.
`docs/ACCESSIBILITY.md` defines the Windows accessibility boundary. **UI
Automation reading is implemented and verified:** Narrator reads an Anodrel
surface aloud on Windows 11, an Inspect cross-check of every earlier flat
published property passes, and the first-party `--uia-property-probe`,
`--uia-focus-probe`, and `--uia-focus-event-probe` verify the fixed current UI
Lab's raw/control-view
property/tree contract, fixed field rectangle, hit target, read-only Value
pattern, absence of a UI Lab Invoke pattern, semantic `SetFocus` result, and
one outbound focus event through real Windows APIs. The Invoke probe also verifies one compiled authenticated-session
button through Windows' standard Invoke pattern, the existing revision-bound
semantic-event mailbox, and its normal child-close sequence. An enabled
authenticated-session button now exposes one bounded Invoke action;
the provider also reports and can move the host's keyboard-focus snapshot
through a bounded UI-thread route, raises one host-only focus-change event for
a real focus move, raises one host-only `ChildrenInvalidated` structure event
after an accepted document replacement, and exposes read-only current field
values. The structure-event probe verifies the fixed root event after one
compiled authenticated document replacement and its normal child-close path.
The first visible overflowing native scroll group now exposes one
host-owned vertical ScrollPattern through the same retained offset as pointer,
wheel, and keyboard input. Its bounded descendants expose the companion
ScrollItem pattern, so an assistive technology can reveal an off-screen item
through that same host-only route; no application can observe or control its
position. Manual
screen-reader activation, focus control and event, field-value,
structure-event, and scrolling verification remain open. Automation editing,
text ranges, Invoke/property/value/text/
selection events, live announcements, and every other pattern remain absent.
See `docs/UI_AUTOMATION_FOCUS.md`, `docs/UI_AUTOMATION_EVENTS.md`,
`docs/UI_AUTOMATION_STRUCTURE_EVENTS.md`, `docs/UI_AUTOMATION_SCROLL.md`, and
`docs/UI_AUTOMATION_SCROLL_ITEMS.md`.
`docs/ACCESSIBILITY_VERIFICATION.md` records the repeatable and hands-on
verification evidence for that Windows accessibility surface.
`docs/UI_AUTOMATION_PROBE.md` defines the fixed host-only Windows property and
tree diagnostic that complements those manual checks.
`docs/UI_AUTOMATION_INVOKE_PROBE.md` defines the separate compiled-session
Windows Invoke acceptance diagnostic.
`docs/UI_AUTOMATION_STRUCTURE_EVENT_PROBE.md` defines the separate
compiled-session Windows structure-event acceptance diagnostic.
`docs/UI_DOCUMENTS.md` defines its exact, capability-free external document
format.
`docs/SCROLLING.md` defines the owned scroll-container boundary, direct
first-viewport Windows scrollbar, accessibility scrolling, and development
diagnostic.
`docs/UI_SESSIONS.md` defines bounded revision and semantic-event state used by
the first capability-checked authenticated UI document replacement path.
`docs/UI_PREVIEW.md` defines the bounded Windows developer preview command.
`docs/UI_SESSION_LAB.md` defines the authenticated native UI delivery smoke
test.
`docs/NATIVE_UI_TEMPLATE.md` defines the accepted development-native UI
template boundary. Its first-party generator creates a typed executable project
that builds and runs through an explicit fixed-grant development host route
without opening a product launch path.
`docs/NATIVE_MENU_TEMPLATE.md` defines the separate development-native menu
template. Its generator and fixed four-grant Windows route exercise the
existing bounded menu protocol without changing the regular template's grants.
`docs/NATIVE_MULTI_WINDOW_TEMPLATE.md` defines the separate development-native
multi-window template. Its generator and fixed five-grant Windows route
exercise the bounded Protocol 1.25 lifecycle without broadening either earlier
template's grants.
`docs/NATIVE_FORM_TEMPLATE.md` defines the separate development-native form
template. Its generator and fixed four-grant Windows route demonstrate
submit-time whole-surface field snapshots without broadening the regular, menu,
or multi-window templates.
`docs/NATIVE_LIVE_STATUS_TEMPLATE.md` defines the separate development-native
live-status template. Its generator and fixed three-grant Windows route
demonstrate three explicit v3 visible-status updates without adding an
accessibility callback, listener check, or delivery result.
`docs/NATIVE_SCROLL_WINDOW_TEMPLATE.md` defines the separate development-native
scroll-window template. Its generator and fixed five-grant Windows route
demonstrate explicit v2 secondary opening and replacement while native scroll
state, input, accessibility behavior, and mappings remain host-owned.
`docs/NATIVE_WINDOW_CONTROLS_TEMPLATE.md` defines the separate development-
native window-controls template. Its generator and fixed eight-grant Windows
route demonstrate every existing targetless session-window control without
broadening any earlier template's authority or exposing native readback.

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

Double-click `start-menu-template.bat` to build a temporary first-party native
menu template and open it through its fixed Windows development route. Choose
**File > Complete menu template session** or press **Ctrl+Shift+M** to exercise
the direct User32 menu, typed event delivery, and clean session shutdown
without Node.js, a webview, or machine policy changes.

Double-click `start-multi-window-template.bat` to build a temporary
first-party multi-window template and exercise a typed secondary window open,
targeted document replacement, tagged action pull, exact secondary close, and
whole-session close. It uses no Node.js, webview, product package, or
machine-policy change.

Double-click `start-scroll-window-template.bat` to build a temporary
first-party scroll-window template. It opens a v2 secondary view, requires
local scrolling to reveal each action, replaces only that view, and then closes
the view and its session. It uses no Node.js, webview, product package, or
machine-policy change.

Double-click `start-window-controls-template.bat` to build a temporary
first-party window-controls template. It visibly exercises the typed title,
size, state, focus, fullscreen, and windowed controls in one host-owned session
without Node.js, a webview, product package, or machine-policy change.

Double-click `start-form-template.bat` to build a temporary first-party native
form template. Enter text and select **Submit form** to exercise host-owned
native entry, semantic submit, one typed whole-surface field snapshot, and
clean close. The generated app does not echo or persist the entered value, and
the walkthrough uses no Node.js, webview, product package, or machine-policy
change.

Double-click `start-network-diagnostic.bat` to build and run the no-window
native HTTPS diagnostic. It tests the complete direct WinHTTP path only against
the compiled `example.com:443` origin; it does not give the sample, templates,
or product fixture general network access.

The public interface and security baseline are documented in docs/PROTOCOL.md
and docs/THREAT_MODEL.md.
