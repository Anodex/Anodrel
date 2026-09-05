# Phase 2: Native host

This is the detailed delivery map for Anodrel's native-host phase. The
[roadmap](../../ROADMAP.md) establishes ordering; the
[Windows release plan](../WINDOWS_RELEASE.md) is authoritative for the current
Windows release gates.

## Scope

Phase 2 establishes a direct native host without a webview or bundled browser
runtime. Windows is the reference implementation. Linux has only the limited
development foundation described below, and macOS work is not started until the
Windows release gates close.

All public application authority crosses the versioned protocol. The host owns
native windows, process lifecycle, UI-thread work, operating-system handles,
machine policy, and private child bootstrap. A capability cannot be added as an
ambient side effect of a native feature.

## Windows foundation delivered

### Lifecycle, launch, and presentation

- Direct Win32 windows, single-instance handling, bounded readiness, immutable
  native-view routing, final-window shutdown, and a multi-window diagnostic.
- Authenticated named-pipe transport with logon-session restriction, one-time
  CNG invitations, and child-only inherited bootstrap delivery.
- Strict no-script application packages with canonical containment and content
  digest verification.
- A registered product-session coordinator that joins verified executable
  launch, private transport, child exit, UI resources, and grouped shutdown.
- A signed launcher route that verifies its selected path, digest, and
  publisher before entering the product-session coordinator.

See [launch](../LAUNCH.md), [product sessions](../PRODUCT_SESSIONS.md), and
[window lifecycle](../WINDOW_LIFECYCLE.md).

### Native UI and accessibility

- A portable semantic document model, strict external format, revision-bound
  session delivery, and host-owned pointer, keyboard, focus, scroll, and
  native-window mappings.
- Narrow session-owned title, state, focus, fullscreen, size, state-read, and
  state-change operations; no handles, geometry, monitor selection, or global
  desktop control cross the protocol.
- Up to four opaque session views with independent documents and action queues.
- Native menu, canonical local shortcut, pointer-originated context menu, tray
  menu, high-contrast palette adaptation, and a direct-rendered scrollbar.
- Windows UI Automation reading, hierarchy, navigation, hit testing, focus,
  Invoke, read-only Value, structure, scroll, and live-status boundaries.

The accessibility provider is verified by focused real-Windows probes and
earlier Narrator/Inspect checks. Current patterns and remaining manual checks
are in [accessibility](../ACCESSIBILITY.md) and
[accessibility verification](../ACCESSIBILITY_VERIFICATION.md).

### Operating-system services

- Bounded Unicode clipboard read/write and validated HTTPS external handoff.
- Direct WinHTTP text fetch with a fixed policy-selected origin set and no
  cookies, redirects, proxy discovery, or automatic authentication.
- File and folder pickers, retained one-use file reads, and separately retained
  text and binary output without exposing filesystem handles or general paths.
- Current-user Credential Manager secrets under the validated application
  identity, one recoverable whole-state snapshot, and host-only paths and crash
  records.
- One-way Shell32 notification and tray entry with no delivery, click, or
  assistive-technology readback.

Each boundary has a protocol contract, explicit grant, direct Windows adapter,
and focused diagnostics. See the corresponding files under `docs/` for exact
limits and manual desktop evidence.

### Rendering and performance

The Windows host presents first-party surfaces with Anodrel's software
rasterizer and one bitmap blit. The portable renderer, brand artwork,
font parser, glyph path/coverage adapter, glyph cache, and text-run foundation
have no third-party or operating-system dependency. The production painter
continues to use GDI text while the owned text path remains a fixed bounded
quality and performance probe.

Release-only frame, startup, idle, transport, and renderer measurements are
maintained under [Performance](../PERFORMANCE.md). The current frame guard
records 6.38 ms average and 8.13 ms worst sustained frame time against a 16 ms
interval; that is a regression guard, not an application comparison claim.

### Distribution and updates

Anodrel's owned Windows distribution foundation can author bounded bundles,
derive manifests from checked bytes, embed them in fresh images, sign them
through direct Windows APIs, privately stage content, publish fixed machine
policy, register a Start-menu launcher and Apps & features entry, recover,
uninstall, and apply a verified newer signed image.

The remaining work requires product authority and real-machine evidence:
production certificate identity and custody, timestamp and endpoint operation,
signed fixture install/update/recovery acceptance, and the first application's
justified UI breadth. See [Windows release readiness](../WINDOWS_RELEASE.md),
[Windows installer](../WINDOWS_INSTALLER.md), and
[product updates](../PRODUCT_UPDATES.md).

## Linux and macOS boundary

Linux has authenticated local transport, private invited-child transport,
direct launcher and child/session lifetime, bounded paths/state/crash stores,
and a fixed Wayland child/view diagnostic. It is not an Anodrel Linux
application host, product launcher, installer, updater, or accessibility
provider.

macOS native-host implementation has not begun. Both platforms remain paused by
the Windows-first delivery decision. Portable work is allowed only where it
directly closes a Windows release gate.

## Acceptance standard

A native capability is not complete merely because a unit test passes. Each
change must have the smallest relevant pure, protocol, integration, and manual
verification. Desktop behavior and machine-trust changes require visible
operator evidence; automation must not claim it observed a result it cannot
observe.
