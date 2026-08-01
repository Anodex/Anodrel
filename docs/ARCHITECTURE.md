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

The current Windows host uses Anodrel-owned modules over direct User32,
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

The Windows host also has an Anodrel-owned Startup Lab. It validates a supplied
application package and performs its internal protocol health check before
composing a branded visual test surface. The lab is host-controlled
diagnostics: it displays only safe validated identity, foundation status, and
this process's own readings; it does not render package text or open a public
pipe client or privileged service. Before the window is created, it performs a
temporary owned loopback through the named-pipe authentication and
`platform.health` path; this is a transport check, not an application session.

The lab also shows every action the platform intends to offer, each carrying a
declared linked or planned state. A planned tile is drawn dimmed, states the
gate it waits on, and is inert; hit-testing and drawing read the same value, so
a tile cannot be enabled by changing its appearance. The two linked tiles open
host-owned windows that display values the host already held, introducing no
capability. See `docs/STARTUP_LAB.md` and Decision 0014.

The owned Windows instance adapter gives the package text surface one bounded,
current-session primary instance per validated application identity. A second
host invocation sends no data: it makes only a best-effort native activation
request to the existing window. The Startup Lab has a separate diagnostic scope
so it does not collide with the application text surface. See
`docs/INSTANCE_LIFECYCLE.md`; product executable identity and a public
second-instance protocol remain separate work.

The direct Win32 host also owns a per-window view registry. Each native handle
maps to one immutable host-created view, and the UI message loop exits only
after the final registered window closes. The `--window-lab` diagnostic proves
this two-window lifecycle without creating a public window-management API. See
`docs/WINDOW_LIFECYCLE.md`.

## Modularity and performance

The dependency direction is one-way: protocol types sit at the center; SDKs and
test hosts depend on the protocol; applications depend only on SDKs and an
injected transport. A mock host must not depend on the SDK at runtime, and no
application may import native host internals.

Public packages should have no import-time side effects, avoid framework-wide
global state, and keep their dependency surface minimal. Production runtime
packages use only Anodrel-owned code, language standard libraries, and direct
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

## Communication model

The application-to-host boundary must use a documented, versioned protocol.
The initial transport may be a local message channel, but the application must
not depend on transport-specific details.

The initial protocol contract is documented in `docs/PROTOCOL.md`. Its SDK,
mock host, and contract tests are transport-neutral; the mock does not select a
native transport implementation. `docs/TRANSPORT.md` defines the owned bounded
frame/session engine, direct one-client Windows named-pipe adapter, and the
separate private child-bootstrap format. The bootstrap adapter can launch a
caller-selected executable but is not integrated with application package trust
or rendered content. The repository's Node-based development sample uses this
path to exercise a real authenticated health request; it remains a diagnostic
client, not a trusted application host. `docs/APPLICATIONS.md` separately
defines the no-script package surface that the Windows host can display. It has
no native bridge or protocol session.

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
