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

The first host is expected to target Windows. Other operating systems should be
added as adapters behind the same service contracts.

The current Windows host uses direct Anodrel modules over User32,
Kernel32, and GDI APIs. Its raw FFI is isolated from the portable protocol and
policy layers. It paints an internal diagnostics view and a bounded,
digest-verified `anodrel.text.v1` application package surface; it has no
webview, script runtime, navigation, or application bridge. macOS and Linux
hosts will follow the same ownership rule through their respective
operating-system APIs.

Drawing is not a host responsibility. Every first-party surface is composed by
two portable crates — `anodrel-canvas`, a software rasterizer, and
`anodrel-brand`, which carries the authored mark as a pre-decoded asset along
with colour tokens and small-size geometry — and reaches the screen through one
bitmap blit. Both crates are free of operating-system and
third-party dependencies and forbid unsafe code, so a future host reuses them
and supplies only three things: a blit, a source of glyph coverage, and a
display-density signal. The host's remaining drawing code is the seam for those
three. See `docs/RENDERER.md` and Decision 0013.

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
a development machine. Production packaging, installation, updates, and a real
signing identity remain separate work. See `docs/SIGNING.md`, `docs/LAUNCH.md`,
`docs/PRODUCT_FIXTURE.md`, and Decisions 0017 through 0020 and 0061.

The Windows paths adapter reads the current user's Local AppData known folder
and passes it to a portable layout builder. That builder derives fixed
per-application `data`, `cache`, and `logs` locations solely from the validated
application identity; it never creates, enumerates, or exposes those paths to
the protocol. The portable state-store foundation reserves one bounded opaque
snapshot below `data`; its direct Windows adapter stages and flushes complete
values before retaining the prior committed state as a recovery candidate.
Protocol 1.10 exposes that service only through separate immediate state read,
replace, and clear grants. The development UI-session diagnostic supplies the
host-derived service explicitly; installed-application policy integration
remains separate work. Logging and future storage services define their own
permission, creation, and recovery rules on top of this layout. See
`docs/PATHS.md`, `docs/STORAGE.md`, Decisions 0021, 0051, and 0052.

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

The two public session-window commands remain deliberately narrower than that
private lifecycle: `window.title.set` proposes a title the host composes with a
validated application name, and `window.state.set` selects one closed minimise,
maximise, or restore action. Both cross a per-session, one-request bridge to
the window's owning UI thread; neither exposes a target, handle, geometry,
current state, focus control, or event. See `docs/WINDOW_TITLE.md`,
`docs/WINDOW_STATE.md`, Decisions 0066 and 0072.

## Modularity and performance

The dependency direction is one-way: protocol types sit at the center; SDKs and
test hosts depend on the protocol; applications depend only on SDKs and an
injected transport. A mock host must not depend on the SDK at runtime, and no
application may import native host internals.

Public packages should have no import-time side effects, avoid framework-wide
global state, and keep their dependency surface minimal. Production runtime
packages use Anodrel code, language standard libraries, and direct
operating-system APIs. Validate messages at a trust boundary rather than
repeatedly in internal layers. The native wire limits encoded messages to 64 KiB
before UTF-8 or JSON parsing and accepts at most four complete frames from one
receive operation; the JSON codec limits nesting to 64 levels. The session
engine runs messages in arrival order and owns a policy-bound core. The Windows
adapter adds logon-SID access control, CNG session credentials, worker-thread
I/O, and a separate bounded one-use bootstrap launcher. The launcher passes
the invitation only over a restricted inherited standard-input handle; it does
not verify executable identity or own a restart policy. Separately,
`anodrel-application` validates a bounded manifest, canonical package paths,
and content digest before the host draws a plain-text application surface.
`anodrel-session-policy` then converts only a validated installed application
record into the identity and machine-selected grants for one `anodrel-core`
host session. It has no operating-system store, launch, pipe, bootstrap, or UI
authority; platform adapters select the record before calling it. A version
1.0 record produces no grants, version 1.1 uses its original strict capability
array, and version 1.2 adds the existing storage, credential, and file grant
names. See Decisions 0023 and 0057.

`anodrel-windows-registered-session` is the Windows composition boundary for
that policy and the owner-restricted named-pipe adapter. Before creating the
endpoint it composes a fixed `HostServices` bundle: identity-derived Windows
state storage and Credential Manager, plus bounded clipboard and HTTPS-link
services. UI-bound file and document services remain unavailable. It returns
an endpoint and a separate sensitive invitation, but does not start a process,
deliver the invitation, or perform pipe I/O. Those remain explicit caller-owned
lifecycle steps.

For an interactive registered session, the same boundary creates one grouped
set of document, input, close, dialog, retained-file, and notification resources
and attaches them to the authenticated transport before the client connects. The
session receives the notification mailbox rather than the Shell32 adapter, so a
worker never holds anything that can reach the notification area; the owning UI
thread performs that call and the adapter's entry lives as long as its view. See
`docs/NOTIFICATIONS.md` and Decision 0062. The Windows
host may consume that group through its internal authenticated-session window
entry point; it cannot be selected or assembled by an application. The group
has no process-launch or native-handle authority. See Decision 0058.

The product-session adapter creates the group before bootstrap delivery, starts
the pipe only on a worker, and observes the tracked child on a separate worker.
Child exit ends pending pipe I/O and closes the native window; pipe exit closes
the window and ends the child. This control path is host-only and has no
protocol message. The host activates it from a command-line route or the
preflight-resolved Startup Lab tile; in both cases the blocking start runs on a
worker and only the resulting grouped resources reach a window. See Decisions
0060 and 0061.

`anodrel-product-fixture` and `anodrel-product-provisioning` are development
tools, not part of the shipped host. The fixture is a first-party child that
speaks only the authenticated protocol; the provisioning helper is the one
component that writes machine policy, and it can write exactly one value for one
compile-time identity after validating the record through the host's own parser.
See `docs/PRODUCT_FIXTURE.md`.

`anodrel-perf-lab` is a development tool, not part of the shipped host. It
measures either the owned in-process wire, authenticated transport, and core
path or the same path through a temporary authenticated Windows named-pipe
loopback, always at fixed 1 KiB and 64 KiB payload sizes. Its bounded iteration
count, fixed warmup, and nearest-rank percentiles make results repeatable; it
does not measure application startup, memory, rendering, or another runtime.
Its report includes only non-identifying target OS, architecture, and available
logical-processor context. See `docs/PERFORMANCE.md` and Decision 0024.

`anodrel-ui` is a portable foundation between an application model and a future
host renderer. It owns only a bounded declarative tree, semantic appearance
roles, layout, clipping, semantic action hit testing, visible accessibility
snapshot, portable focus traversal, and host-retained vertical scroll state.
`anodrel-ui-document` separately owns
the exact, bounded JSON interchange form for that tree. The Windows UI Lab
decodes a fixed compile-time fixture through it, and the separate explicit
Windows developer preview can render one bounded operator-selected document.
`anodrel-ui-session` owns the revision-bound state behind one authenticated
transport session's capability-checked 24 KiB document replacement operation;
its single-slot latest-document mailbox can transfer a snapshot to another host
thread without queueing or I/O, while its separate 32-candidate input mailbox
keeps host-layout-derived actions bounded until the authenticated
`ui.events.read` pull validates them. The development-only Windows UI Session
Lab consumes both supplied mailboxes in one host-created view. It renders an
explicit v2 scroll tree with locally retained offsets driven only by native
wheel and page input; those offsets never enter the protocol. It is not a
public window capability and delivers no native command. None of these crates
has operating-system authority. A separate coalescing close signal lets the
host end only that same authenticated session after a capability-checked
request; it carries no window target or native handle. The
host remains responsible for text measurement, mapping appearance roles to an
actual visual system, rendering, input delivery, operating-system accessibility
and focus adapters, and every capability decision. See `docs/UI.md`,
`docs/UI_DOCUMENTS.md`, `docs/UI_SESSIONS.md`, `docs/UI_PREVIEW.md`,
`docs/SCROLLING.md`, and Decisions 0025 through 0039.

`anodrel-windows-accessibility` sits directly above that snapshot. It is a pure
mapping from one visible snapshot to the values Microsoft UI Automation asks
for: control types, property values, runtime IDs, and screen-space rectangles.
It performs no operating-system call and cannot fail. `anodrel-windows-uia`
publishes that mapping through `WM_GETOBJECT`, a root, flat child navigation,
properties, and hit testing. An enabled button in a current authenticated UI session
also exposes the one `Invoke` pattern: it offers the exact revision-bound
semantic action candidate to the existing session mailbox, with no native input
message, application callback, focus movement, or accessibility-specific queue
(Decision 0069). The same provider reports its copied host-focus snapshot
through `GetFocus` and `HasKeyboardFocus`, without `SetFocus`, events, or a
live-state lookup (Decision 0070). A matching visible field separately exposes
only a copied current host value through a read-only `IValueProvider`; it has no
automation write, caret, selection, text range, or event route (Decision 0071).
No other pattern is supplied. The boundary does not read the tree back or reveal
that assistive technology is present. Narrator and an Inspect cross-check
verified the read provider on Windows 11; manual screen-reader activation,
focus, and field-value verification remain open. See `docs/ACCESSIBILITY.md`,
Decisions 0063, 0069, 0070, and 0071.

## Communication model

The application-to-host boundary must use a documented, versioned protocol.
The initial transport may be a local message channel, but the application must
not depend on transport-specific details.

The initial protocol contract is documented in `docs/PROTOCOL.md`. Its SDK,
mock host, and contract tests are transport-neutral; the mock does not select a
native transport implementation. `docs/TRANSPORT.md` defines the bounded
frame/session engine, direct one-client Windows named-pipe adapter, and the
separate private child-bootstrap format. The bootstrap adapter can launch a
caller-selected executable but is not integrated with application package trust
or rendered content. The repository's Node-based development sample uses this
path to exercise a real authenticated health request; it remains a diagnostic
client, not a trusted application host. Its Protocol 1.11 diagnostic read
projects only the host-supplied closed catalogue through the existing
`diagnostics.read` grant; no transport layer can manufacture dynamic diagnostic
content. `docs/APPLICATIONS.md` separately
defines the no-script package surface that the Windows host can display. It has
no native bridge or protocol session.

The native session handles the existing `cancel` control before the core begins
a matching request. It retains no more than 32 unresolved opaque IDs, sends no
control response, and cannot interrupt work already begun. See
`docs/TRANSPORT.md` and Decision 0054.

Every request should have:

- protocol version;
- request ID;
- operation name;
- capability context;
- typed payload;
- cancellation identity where applicable.

Every response should have:

- request ID;
- success or failure status;
- typed result or structured error;
- diagnostic metadata safe to expose to the application.

Events must be explicitly subscribed to and must identify their source and
schema version. Protocol changes require compatibility tests and a decision
record when they affect existing consumers.

## Security model

Security is a platform responsibility, not a UI convention.

- Capabilities are explicit and least-privilege.
- Sensitive operations require a policy decision before execution.
- Application content cannot gain native access merely because it is rendered.
- File paths are validated at the host boundary.
- Secrets are kept in operating-system credential storage where available.
- Child processes are tracked, bounded, and terminated during shutdown.
- Logs must not contain credentials or raw secret material.
- Native APIs are exposed through narrow operations, not arbitrary host access.

The detailed threat model will be added before the first host exposes filesystem,
process, or credential operations.

`docs/THREAT_MODEL.md` establishes the current protocol baseline. It must be
extended with the selected native-host, UI, and transport controls before any
privileged capability is implemented.

## Data and storage

Anodrel should provide paths and secure storage primitives, but applications
should own their domain data format. The platform must not create a shared
database that couples unrelated applications together.

Application data should be:

- versioned;
- exportable;
- recoverable after interrupted writes;
- isolated by application identity;
- excluded from source control by default.

## Migration strategy

Migration follows the strangler pattern:

1. Define platform contracts without copying Anodex code.
2. Add an adapter around the existing Anodex behavior.
3. Keep the current Electron path working.
4. Build a native host against the same contracts.
5. Compare the two hosts with shared integration tests.
6. Move Anodex features one boundary at a time.
7. Remove Electron only after feature parity and recovery are demonstrated.

No migration step should require a big-bang rewrite or make the existing Anodex
repository unbuildable.

## Testing strategy

- Unit tests cover protocol types, validation, permissions, and pure logic.
- Contract tests run the same application requests against mock and native hosts.
- Integration tests cover lifecycle, dialogs, paths, secure storage, and process
  cleanup.
- Security tests cover traversal, capability bypass, malformed messages, and
  shutdown races.
- Manual smoke tests cover window behavior and operating-system integration.

## Open decisions

The following choices remain intentionally open and must be recorded before
implementation locks them in:

- packaging, signing, and update strategy;
- license and contribution model.
