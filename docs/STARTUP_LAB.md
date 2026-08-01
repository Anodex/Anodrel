# Anodrel Startup Lab

**Status:** Windows foundation test surface.

## Purpose

The Startup Lab is Anodrel's first branded native surface. It is a first-party
Win32 diagnostic screen, not an Electron clone, a web page, or an application
renderer. It gives a developer a fast visual smoke test and shows, in one look,
which parts of the platform are real.

The lab uses only Anodrel-owned Rust, the owned renderer described in
`docs/RENDERER.md`, and direct User32/GDI/Kernel32 calls. It contains no
webview, browser engine, JavaScript, external resource fetch, link, navigation,
script execution, or native bridge.

## Launch contract

The Windows host accepts:

~~~text
anodrel-windows-host --showcase <anodrel.application.json>
~~~

Before creating a window, this route must:

1. validate the supplied application package through `anodrel-application`;
2. perform the host's internal `platform.health` protocol-core check;
3. complete one temporary, owned named-pipe loopback: authenticate the first
   `ANDR` frame and make one `platform.health` request; and
4. fail closed with a safe error if any check fails.

`start.bat` launches the sample manifest through this route. The screen
therefore verifies the sample package's canonical containment and SHA-256
content digest in the normal developer startup path.

`start.bat` builds in **release**. The surface composes every frame in software,
and an unoptimised build cannot hold the reveal's frame rate. This is a
requirement of the route, not a preference — see `docs/RENDERER.md`.

## What the screen demonstrates

### Status cards

| Card | Actual condition | What it does not claim |
| --- | --- | --- |
| Owned Core | The host constructed and processed its internal `platform.health` request. | A public application session or a privileged capability. |
| Verified Package | The supplied manifest and its bounded text content passed containment and digest checks. | Publisher signing or verified executable identity. |
| Private IPC | A temporary current-session pipe accepted an owned in-process loopback client, which authenticated and completed one `platform.health` round trip. | A public application client, executable launch, bootstrap selection, or any privileged capability. |
| Native Shell | The host created and owns this Win32 window and composed this frame with its own renderer. | A public window API, an application window capability, or a native bridge. |

### Action strip

Every tile the platform intends to offer is shown, in one of two declared
states. The state is data on the tile; hit-testing and drawing read the same
value, so a tile cannot be enabled by changing how it looks. Decision 0014
records the reasoning.

| Tile | State | Behaviour |
| --- | --- | --- |
| Launch Sample | **Planned** — needs verified executable identity | Dimmed, labelled, inert. |
| Open Logs | **Linked** | Opens a host-owned view of the safe typed startup-event ledger. |
| Inspect Package | **Linked** | Opens a host-owned window showing the verified package facts. |
| Runtime Diagnostics | **Linked** | Opens a host-owned window showing protocol, transport, and process readings. |

A planned tile states the gate it is waiting on instead of a description, does
not respond to hover, and takes no pointer cursor. `ROADMAP.md` tracks each one
against the decision that gates it.

The three linked tiles introduce no capability. They display values the host
already held after startup: the package's identity, declared relative content
path, verified digest, byte count, and documented limits; and the protocol
version, request and frame limits, JSON depth limit, pipe scope, working set,
startup time, and last frame cost. The log view is limited to the typed events
defined in `docs/LOGGING.md`; it cannot show application text, paths, secrets,
raw native errors, or persistent history.

### Footer readings

`Runtime` is the host's package version. `Memory` is this process's working set.
`Startup` is the time from process start to the surface being ready. `Integrity`
reflects the package checks that already passed. The right-hand reading reports
the latest full-composition measurement captured before the settled surface is
cached — the renderer describing itself. It is a visual diagnostic, not a
continuous profiler; resizing or interacting with the surface refreshes it.

All are measurements of this process. Nothing reads another process, the
filesystem, or any user data.

## What is never shown

The title and identity line come from the validated manifest. Application
content text, untrusted request values, credentials, canonical filesystem paths,
and native error details are never rendered on the Startup Lab surface.

The window layer receives a `PackageFacts` copy rather than the package itself,
so it cannot reach a resolved filesystem path or any value that skipped
validation. The manifest-relative content path may be displayed; the path it
resolves to may not.

## Visual contract

The screen owns its complete client-area drawing. It is composed into an owned
canvas and reaches the screen in a single blit, so a partial frame is never
visible.

- deep near-black backdrop, a wide radial bloom behind the hero, and faceted
  corner planes at a few units of alpha;
- the Anodrel mark drawn from the authored artwork, with its glow taken from
  the artwork's own alpha channel;
- the wordmark filled with the mark's own ramp, so identity reads the same in
  type as in geometry;
- status cards and action tiles on panels with hairline borders;
- a footer strip of process readings.

The layout is authored against a base size of 1240×900 logical pixels and
scaled. The scale is driven by the smaller axis, so nothing overflows when the
window is resized away from its designed aspect ratio, and the minimum client
size is enforced at 900×660. The process is per-monitor DPI aware, so the same
code serves every display density.

### Reveal

Opening the surface plays a single reveal of about 1.3 seconds: the header
settles, the mark scales up from slightly small and fades in, the title and
identity rise, then the cards and action tiles stagger in. Stages overlap
deliberately, so it reads as one motion rather than a queue.

The reveal is a one-shot. Once it settles, only the mark keeps a slow ambient
motion at 30 frames per second; the host repaints its declared region rather
than the whole surface, and pauses that timer while inactive or minimized.

The design deliberately borrows the useful role of Electron's welcome screen: a
first-run orientation and a visual test point. Its mark, palette, wording,
layout, motion, and implementation are Anodrel's own.

## Manual verification

From the repository root, double-click `start.bat`. Confirm that an **Anodrel
Startup Lab** window opens and shows:

- the Anodrel mark in the taskbar and title bar, generated at run time from the
  brand crate rather than from a compiled icon resource;
- the `org.anodrel.sample` identity below the hero mark;
- Owned Core, Verified Package, Private IPC, and Native Shell as ready;
- Launch Sample dimmed and marked `PLANNED`;
- Open Logs, Inspect Package, and Runtime Diagnostics highlighting under the
  pointer, taking a hand cursor, and each opening a host-owned window when
  clicked;
- a smooth reveal that settles into low-frequency mark motion.

Resize the window and confirm the layout scales without overflowing and without
flicker. Close the Startup Lab window last: the host exits only after its final
window closes.

A changed or invalid manifest/content pair must prevent the Startup Lab window
from opening. A failed private IPC handshake or health round trip must also
prevent it from opening.

## Automated coverage

Manual verification covers what only a real window can show. Everything else is
asserted:

- every region stays inside the client area at the minimum, designed, and a
  large client size;
- cards are ordered left to right without overlapping;
- every action tile is hit-testable at its own centre, and points outside the
  strip hit nothing;
- hit-testing follows the layout when the window is resized;
- exactly the tiles with a documented host operation behind them are marked linked;
- the linked log action displays only the closed host event catalogue;
- the reveal adds content over time and is completely static by the time its
  timer stops;
- a frame composes inside the animation timer's interval in a release build.
