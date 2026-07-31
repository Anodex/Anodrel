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

## Communication model

The application-to-host boundary must use a documented, versioned protocol.
The initial transport may be a local message channel, but the application must
not depend on transport-specific details.

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

- native host language and framework;
- first transport implementation;
- webview versus native UI strategy;
- Windows-only first release scope;
- packaging, signing, and update strategy;
- license and contribution model.
