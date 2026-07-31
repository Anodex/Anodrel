# Anodrel Roadmap

This roadmap is intentionally staged. The platform must become useful and
stable before Anodex depends on it.

## Phase 0 — Foundation

Status: **In progress**

- Create the standalone repository.
- Establish folder ownership and documentation rules.
- Define the initial architecture and security boundaries.
- Record the repository-separation decision.
- Choose the first supported operating system. **Completed: Windows.**
- Define the first direct Windows API host. **Completed:** an owned Win32
  window, JSON codec, protocol core, and lifecycle smoke test (Decision 0006).
- Define a bounded application-to-host frame and session engine. **Completed:**
  owned wire framing and host session limits (Decision 0007).
- Implement the authenticated Windows named-pipe adapter. **Completed:**
  logon-SID-restricted one-client adapter with CNG invitation (Decision 0008).
- Define controlled application-content hosting and private invitation delivery.

Acceptance gate: the project has an agreed architecture, a documented first
milestone, and no dependency on Anodex source code.

## Phase 1 — Contracts and protocol

Status: **Foundation slice in progress**

- Define the platform service interfaces.
- Define protocol envelopes, request IDs, errors, cancellation, and events.
- Define capability and permission declarations.
- Create compatibility and schema tests.
- Build a minimal mock host for application development.

Acceptance gate: a small sample application can communicate with the mock host
using only documented interfaces.

The initial protocol, SDK, mock host, sample application, contract suite, and
bounded native session engine are established. Phase 2 does not begin until the
remaining Phase 0 adapter/content decisions and the threat-model gate are
complete.

## Phase 2 — Native host

Status: **Direct Windows host in progress**

- Create the first native host, beginning with Windows.
- Implement lifecycle and single-instance behavior.
- Implement window creation and controlled application content loading.
- Implement file dialogs, external links, clipboard, notifications, and paths.
- Implement secure credential storage through the operating system.
- Add logging, crash reporting boundaries, and shutdown behavior.

Acceptance gate: a sample application can run without Electron and exercise the
core platform services safely.

The direct Windows host creates and paints an Anodrel-owned Win32 window and
validates the core protocol shape under Decision 0006. Decision 0007 adds the
bounded framing and session engine. Decision 0008 adds the authenticated direct
Windows named-pipe adapter. Remaining acceptance work begins with controlled
application-content hosting, private invitation delivery, and operation-specific
native tests.

## Phase 3 — Reusable SDK and tooling

Status: **Planned**

- Provide a small application SDK.
- Provide development and diagnostic tools.
- Document packaging, signing, updates, and compatibility.
- Add examples for a desktop application and a command-line application.

Acceptance gate: a new project can be created from the documented template and
run without knowing the internals of the native host.

## Phase 4 — Anodex adapter

Status: **Planned**

- Connect Anodex through the public Anodrel interfaces.
- Keep Anodex's existing Electron adapter working during migration.
- Move platform-specific operations behind the new adapter.
- Compare behavior and performance between the old and new hosts.
- Switch Anodex only after feature parity and recovery procedures are proven.

Acceptance gate: Anodex can run on Anodrel without importing Electron APIs
from its core application logic.

## Phase 5 — Additional applications and platforms

Status: **Planned**

- Add a second sample application.
- Add macOS and Linux host adapters as resources allow.
- Stabilize the protocol and publish SDK documentation.
- Define a long-term release and support policy.

## Explicitly deferred

- Native UI rewrite.
- Custom browser engine.
- Custom operating system.
- Full Anodex rewrite before the platform contracts are proven.
