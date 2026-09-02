# Anodrel Architecture

## Purpose

Anodrel is a reusable application platform, not an AI product. It owns the
boundary between an application and the operating system while allowing an
application to choose its own domain logic and user experience.

Anodex is the first planned consumer, but no layer in Anodrel should depend
on Anodex concepts such as conversations, models, projects, or AI providers.

## Layer model

~~~text
┌─────────────────────────────────────────────────────────┐
│ Application                                             │
│ Anodex, future desktop apps, command-line apps         │
└──────────────────────────────┬──────────────────────────┘
                               │ documented SDK/protocol
┌──────────────────────────────▼──────────────────────────┐
│ Platform Core                                            │
│ lifecycle, capabilities, messages, errors, cancellation  │
└──────────────────────────────┬──────────────────────────┘
                               │ platform service interfaces
┌──────────────────────────────▼──────────────────────────┐
│ Native Host                                              │
│ windows, storage, dialogs, processes, notifications     │
└──────────────────────────────┬──────────────────────────┘
                               │ operating-system APIs
┌──────────────────────────────▼──────────────────────────┐
│ Operating System                                         │
│ Windows first; macOS and Linux later                    │
└─────────────────────────────────────────────────────────┘
~~~

## Responsibilities

### Application layer

Applications own their domain behavior, screens, workflows, data models, and
product-specific policies. They request platform capabilities through the SDK
or protocol and do not reach directly into native host internals.

### Platform Core

The core owns concepts that are useful to multiple applications:

- application lifecycle contracts;
- capability discovery and permission requests;
- request/response/event envelopes;
- cancellation and shutdown behavior;
- structured errors and diagnostics;
- protocol version negotiation;
- host health and compatibility checks.

The core must remain independent from Electron, a specific frontend framework,
and any single operating system.

### Native Host

The native host owns operating-system integration:

- process and window lifecycle;
- single-instance behavior;
- file and folder dialogs;
- secure storage;
- notifications;
- clipboard and external links;
- application paths;
- child-process management;
- update and packaging integration;
- native security and isolation controls.

The first host targets Windows. Linux now has a shared local-transport adapter,
a separately tested private child transport, a direct development child
launcher, one host-owned development child/transport lifecycle, bounded direct
state and crash stores, and one fixed direct-Wayland diagnostic surface with a
host-owned local pointer probe. A separate development-window Lab retains that
fixed view with one private child session; its application desktop host and
most operating-system services remain later work.
Other operating systems should be added as adapters behind the same service
contracts.

The current Windows host uses direct Anodrel modules over User32,
Kernel32, and GDI APIs. Its raw FFI is isolated from the portable protocol and
policy layers. It paints an internal diagnostics view and a bounded,
digest-verified `anodrel.text.v1` application package surface; it has no
webview, script runtime, navigation, or application bridge. The Linux transport
adapter uses direct abstract Unix sockets and same-UID peer verification. Its
fixed ANLI child proof opens only a host-issued endpoint, while the direct
development launcher starts only a host-selected exact executable with private
standard input. The Linux development-session adapter owns one such child and
its authenticated worker until either ends. Its separate development-window
Lab closes one fixed Wayland view when that session ends, but creates no Linux
application identity or product service. A direct Linux paths
adapter derives an effective-user default data root before applying the
portable layout. The direct Linux state adapter uses that host-owned layout
only to retain one bounded recoverable snapshot through private directory
descriptors; its direct crash store uses the sibling host logs location for
closed panic records only. The Linux Wayland lab is a separate direct desktop
protocol adapter that presents a fixed first-party canvas through two bounded
shared-memory mappings and consumes one local diagnostic activation. The
development-window Lab owns its child/view lifetime only; neither has an
application, IPC, or product-launch route.
None exposes a filesystem capability. macOS and a Linux application host will
follow the same ownership rule through their respective operating-system APIs.

Drawing is not a host responsibility. Every first-party surface is composed by
two portable crates — `anodrel-canvas`, a software rasterizer, and
`anodrel-brand`, which carries the authored mark as a pre-decoded asset along
with colour tokens and small-size geometry — and reaches the screen through one
bitmap blit. Both crates are free of operating-system and
third-party dependencies and forbid unsafe code. `anodrel-font` is the
separate first-party parsing foundation for a future host-owned glyph source:
it maps Unicode to a glyph ID, reads bounded simple contours from already-owned
memory, validates bounded horizontal metrics, and converts contours to exact
quadratic paths, but does not load or draw a font. `anodrel-glyph` is the equally portable, separate adapter that converts
one such path through an explicit device transform into a bounded canvas
polygon and then one bounded coverage mask; it does not parse or draw a font.
A future host therefore still supplies a blit and a display-density signal. See
`docs/RENDERER.md`, `docs/FONTS.md`, `docs/GLYPH_RENDERING.md`, Decisions 0013,
0133 through 0138.

The Windows host also has an Anodrel Startup Lab. It validates a supplied
application package and performs its internal protocol health check before
composing a branded visual test surface. The lab is host-controlled
diagnostics: it displays only safe validated identity, foundation status, and
this process's own readings; it does not render package text or open a public
pipe client or privileged service. Before the window is created, it performs a
temporary internal loopback through the named-pipe authentication and
`platform.health` path; this is a transport check, not an application session.

The lab also shows every action the platform intends to offer, each carrying a
declared linked or planned state. A planned tile is drawn dimmed, states the
gate it waits on, and is inert; hit-testing and drawing read the same value, so
a tile cannot be enabled by changing its appearance. Its three linked tiles
open native windows that display values the host already held, introducing
no capability. The log view reads a bounded typed event ledger that cannot
accept dynamic application or native diagnostic text. Its one launch tile is the
exception to the compile-time rule: its state comes from a verification-only
preflight that runs before the surface opens, and drawing, hover, and
hit-testing all read that single value, so the tile cannot be live on a machine
where the record or signature does not validate. See `docs/STARTUP_LAB.md`,
`docs/LOGGING.md`, `docs/PRODUCT_FIXTURE.md`, Decisions 0014, 0016, and 0061.

The Windows instance adapter gives the package text surface one bounded,
current-session primary instance per validated application identity. A second
host invocation sends no data: it makes only a best-effort native activation
request to the existing window. The Startup Lab has a separate diagnostic scope
so it does not collide with the application text surface. See
`docs/INSTANCE_LIFECYCLE.md`; product executable identity and a public
second-instance protocol remain separate work.

The Windows signature adapter is a separate operating-system boundary. It asks
Windows Authenticode policy to verify an embedded executable signature and
returns only the leaf certificate fingerprint from a successful trust state.
It neither trusts the mutable package directory nor launches the executable.
The installed application-record foundation binds the expected executable
digest and signer fingerprint to a validated package identity in a record
outside that package. The policy-store adapter reads that record only from a
fixed, machine-wide 64-bit Windows registry location using query access. The
host-only launch service locks the executable, rechecks containment and digest,
checks Authenticode and the publisher fingerprint, creates only the exact
argument-free `.exe`, and returns a child handle that terminates on host
shutdown. `anodrel-windows-product-session` now joins that child, one
registered interactive pipe, and one grouped native UI session under one
host-owned lifetime. A separate verification-only entry point runs the same
pre-launch sequence without creating a process, so a surface can decide whether
a launch is currently possible. A development-only signed fixture and a
controlled provisioning helper — both outside the host, which never writes
machine policy, installs trust, or signs anything — exercise the joined path on
a development machine. The separate owned installer foundation now carries a
strict resource-bearing release envelope through private staging, extracted
publisher verification, no-overwrite promotion, fixed policy publication,
recovery, uninstall, a refreshed publisher-and-forward-version update
transaction, and a one-record policy-backed rollback. The release authoring
boundary creates bounded owned bundles, derives strict manifests from those
checked bundle bytes, embeds them into fresh images, and signs only fresh checked
image copies with one explicit current-user certificate through Windows
Authenticode. Its no-argument machine routes select only their
  current signed release and fixed 64-bit Program Files root. The owned update
  route now discovers its catalogue from signed installed policy, streams and
  locks a matching image, then asks Windows to elevate only its fixed update
  command. One opaque native updater now composes cache recovery, signed
  discovery, private download, image locking, and that handoff. Its fixed
  current-user cache remains separate from every application filesystem surface.
  Production certificate custody, timestamping, a signed positive acceptance
  run, user-visible consent/progress, and a real production identity remain
  separate work.
See `docs/SIGNING.md`, `docs/RELEASE_MANIFEST.md`, `docs/LAUNCH.md`,
  `docs/PRODUCT_FIXTURE.md`, `docs/WINDOWS_INSTALLER.md`,
  `docs/UPDATE_HANDOFF.md`, `docs/UPDATE_CACHE.md`, `docs/UPDATE_FLOW.md`, and
  `docs/UPDATE_ACCEPTANCE.md`, and Decisions 0017 through 0020, 0061, and 0140
  through 0172.

The Windows paths adapter reads the current user's Local AppData known folder
and passes it to a portable layout builder. The Linux paths adapter obtains the
same kind of root from its effective account. That builder derives fixed
per-application `data`, `cache`, and `logs` locations solely from the validated
application identity; it never creates, enumerates, or exposes those paths to
the protocol. The portable state-store foundation reserves one bounded opaque
snapshot below `data`. Its direct Windows adapter stages and flushes complete
values before retaining the prior committed state as a recovery candidate; its
direct Linux counterpart opens and creates only private fixed-name files
through directory descriptors and retains the same one backup. Protocol 1.10
exposes that service only through separate immediate state read, replace, and
clear grants. The development UI-session diagnostic supplies the Windows
host-derived service explicitly; installed-application policy integration
remains separate work. Logging and future storage services define their own
permission, creation, and recovery rules on top of this layout. See
`docs/PATHS.md`, `docs/STORAGE.md`, Decisions 0021, 0051, 0052, and 0125.

The Windows credential adapter stores a bounded secret only under the exact
target derived from a validated application identity and credential name. It
uses the current user's generic Credential Manager store. Protocol 1.12 reaches
an injected identity-bound credential service only through separately granted
exact read, write, and delete operations; it exposes no enumeration, arbitrary
target, renderer, diagnostic, or identity field. The portable secret and target
types redact their contents. A development UI-session diagnostic supplies the
service from its Windows pipe worker; installed policy and consent remain
separate product gates. See `docs/CREDENTIALS.md`, Decisions 0022 and 0056.

The direct Win32 host also owns a per-window view registry. Each native handle
maps to one immutable host-created view, and the UI message loop exits only
after the final registered window closes. Each window message runs inside a
panic-containment boundary, because the callback is `extern "system"` and an
escaping panic would abort the process without running a destructor — stranding
a verified product child. A contained panic ends the loop instead, and the
ordinary drop paths perform the cleanup. The `--window-lab` diagnostic proves
this two-window lifecycle without creating a public window-management API. See
`docs/WINDOW_LIFECYCLE.md`.

The existing primary-only session-window commands remain deliberately narrower
than the private lifecycle: `window.title.set` proposes a title the host
composes with a validated application name, `window.state.set` selects one
closed minimise, maximise, or restore action, and `window.state.get` returns
one separately granted pull-only closed-state snapshot.
`window.state.changes.read` consumes one separately granted coalesced later
state or `null`. `window.focus.request`
asks Windows to foreground that same session-owned window.
`window.fullscreen.set` chooses only reversible borderless fullscreen or
windowed restoration for that same window; `window.size.set` chooses only a
bounded logical client area for it. All cross a per-session, one-request bridge
to the window's owning UI thread; none exposes a target, handle, geometry,
monitor, display mode, focus readback, input, retry, event, position, DPI, or
bounds readback. `window.state.get` has no future change notification and may
be stale immediately; `window.state.changes.read` has no callback, wait,
history, timestamp, or subscription. The focus request does not control semantic UI focus or
bypass Windows foreground policy; fullscreen retains its native placement facts
privately and is not exclusive display control.

Protocol 1.25 adds a separate bounded exception: `window.open` and
`window.close` address only opaque views in the authenticated session's own
four-view group. The portable group validates documents, revisions, and input
queues; the direct Windows host performs all logical-ID-to-HWND resolution on
the UI thread through a private map. `ui.document.replace.window` can update
`main` or a known secondary, and `ui.events.read.window` returns only
revision-checked actions tagged by their logical view. Neither operation
enumerates, looks up, or observes a native window. See
`docs/MULTI_WINDOW.md`, `docs/WINDOW_TITLE.md`, `docs/WINDOW_STATE.md`,
`docs/WINDOW_STATE_OBSERVATION.md`, `docs/WINDOW_STATE_CHANGES.md`, `docs/WINDOW_FOCUS.md`,
`docs/WINDOW_FULLSCREEN.md`, `docs/WINDOW_SIZE.md`, and Decisions 0066, 0072,
0117, 0118, 0085, 0086, 0088, 0092, and 0093.

Protocol 1.26 adds parallel exact-v3 document operations for the primary and
these session-owned views. A v3 document may carry one visible semantic status.
The Windows host may emit one best-effort accessibility event after a later
changed visible status reaches an established view, but that is never a
protocol response or application-observable delivery channel. See Decision
0100.

Protocol 1.27 adds parallel exact-v2 opening and replacement operations for
scroll-only secondary views. Each view retains its own host-local position and
Windows scrolling behavior; no position, callback, event, or native mapping
crosses the protocol. See Decision 0102.


## Further architecture detail

The project-wide rules for modularity, performance, communication, security, data, migration, and testing are maintained in [Architecture foundations](ARCHITECTURE_FOUNDATIONS.md).
