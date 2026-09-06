# Foundation status

This page is the concise, maintained picture of Anodrel's implemented
foundation. The [Windows release plan](WINDOWS_RELEASE.md) remains the source
of truth for release gates; the [roadmap](../ROADMAP.md) records delivery order.

## Position

Anodrel is in foundation implementation. Windows is the reference platform and
is about 72% through its release goal. The wider Windows, Linux, and macOS
programme is about 30% complete. Linux desktop work and macOS work remain
paused until the Windows reference release closes its gates.

The project is a native-first application platform, not an Electron clone or an
application. It supplies a documented, versioned boundary between application
code and direct operating-system services without shipping a browser engine,
webview, Node.js runtime, or copied Anodex source.

## Working Windows foundation

The current Windows foundation includes:

- a transport-neutral protocol, typed SDK, mock host, samples, and compatibility
  suite;
- authenticated named-pipe sessions with one-time child-only bootstrap delivery;
- a direct Win32 host with bounded native windows, lifecycle control, and a
  branded Startup Lab smoke test;
- a strict package format and verified executable launch path: canonical
  containment, content digests, Authenticode, machine policy, locked child
  launch, verified launcher, and grouped shutdown;
- an owned software renderer, a portable rasterizer and brand crate, and
  bounded first-party font, glyph, glyph-cache, and text-run foundations;
- host-owned native UI documents, input, scrolling, menus, context menus, tray
  menus, multiple session views, high-contrast adaptation, and narrow
  session-window controls;
- Windows UI Automation reading, navigation, hit testing, focus, Invoke,
  read-only Value, structure, scroll, and live-status boundaries; and
- direct Windows adapters for bounded clipboard, HTTPS handoff and text fetch,
  file and folder selection, one-use file output, credentials, state, shell
  notification, and application paths.

Each service sits behind an explicit grant or host-owned boundary. Applications
do not receive raw operating-system handles, ambient filesystem authority,
global shortcuts, an accessibility listener, process control, or a browser
bridge.

## Session and application surface

Authenticated sessions can use only capabilities that the validated installed
record grants. The current capabilities cover bounded state, credentials,
clipboard text, HTTPS text fetch, file/folder selection, retained one-use text
and binary output, native menus, semantic context and tray menus, bounded
multi-window sessions, scrolling, and narrow window title, state, focus,
fullscreen, size, and state-observation operations.

The UI surface is deliberately semantic. Applications submit strict documents
and read revision-bound semantic actions; the host retains pointer data,
coordinates, native window mappings, focus mechanics, scroll positions, and
accessibility-provider state. The exact current boundaries live in the protocol
and feature documents, especially [UI](UI.md), [UI sessions](UI_SESSIONS.md),
[window lifecycle](WINDOW_LIFECYCLE.md), and [accessibility](ACCESSIBILITY.md).

## Release foundation

Anodrel owns its Windows release path. It can author bounded release bundles,
derive strict manifests, embed them in a fresh image, sign a current checked
image through direct Windows APIs, validate its own embedded release, privately
stage, install, register a Start-menu launcher and Apps & features entry,
recover, uninstall, and perform a guarded signed update transaction.

The update route starts from signed installed policy, accepts a strictly newer
candidate from its fixed catalogue source, verifies the candidate before
elevation, and proves machine policy after the elevated transaction. It retains
one fixed prior record for a separate verified recovery command. No installer
framework, archive format, web runtime, or third-party desktop runtime is part
of that route.

A fixed development fixture exercises the joined verified child, launcher, and
native-session design. A separate installed development fixture prepares the
full signed installer chain. They are test harnesses, not a production identity
or product release. Running either positive path changes machine trust and
therefore remains an explicit operator action.

## Evidence and remaining work

Release-only automated evidence currently includes a 6.37 ms average and
8.09 ms worst sustained frame time against a 16 ms interval, startup reporting,
an idle-window report, native workspace tests, and contract coverage. These are
guards, not a substitute for visible Windows behavior.

The remaining Windows work is concentrated in:

1. manual Windows desktop acceptance—native menus, pickers, window behavior,
   screen-reader interactions, tray/notification behavior, and signed fixture
   flows;
2. production signing identity, certificate custody, renewal, timestamping, and
   the release operation, all of which require product authority; and
3. first-application requirements that justify additional reusable UI controls
   or platform capabilities.

The project does not claim feature-for-feature Electron parity. It is building a
smaller, direct, security-conscious native platform with only capabilities that
have a defined owner, contract, and verification path.

## Related documents

- [Windows release readiness](WINDOWS_RELEASE.md)
- [Performance plan](PERFORMANCE.md)
- [Architecture](ARCHITECTURE.md)
- [Protocol](PROTOCOL.md)
- [Development product fixture](PRODUCT_FIXTURE.md)
- [Installed development fixture](INSTALLED_PRODUCT_FIXTURE.md)
- [Windows installer](WINDOWS_INSTALLER.md)
- [Product updates](PRODUCT_UPDATES.md)
- [Accessibility](ACCESSIBILITY.md)
