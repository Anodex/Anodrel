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
- Implement private invitation delivery. **Completed:** bounded `ANBI` record
  over a child-only inherited handle (Decision 0009).
- Define controlled application-content hosting and application identity.
  **Completed for the first no-script package surface:** strict manifest,
  canonical containment, owned SHA-256 verification, and direct Win32 text
  rendering (Decision 0010). Publisher trust and executable identity remain
  required before product process launch.

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
bounded native session engine are established. The minimum Phase 0 content
boundary and associated threat-model controls are complete. Product launch,
publisher trust, and a capability bridge remain later native-host gates.

## Phase 2 — Native host

Status: **Direct Windows host in progress**

- Create the first native host, beginning with Windows.
- Implement lifecycle and single-instance behavior. **Completed for the first
  Windows package text surface and Startup Lab:** current-session primary
  instance claim, bounded readiness, and no-data activation request (Decision
  0011). A verified product executable lifecycle remains required.
- Implement window creation and controlled application content loading.
  **Completed for host-owned Windows surfaces:** per-window immutable view
  routing, final-window shutdown, a two-window diagnostic, and the verified
  no-script text package surface (Decision 0012). A public application window
  capability remains deferred.
- Draw first-party surfaces with an owned renderer. **Completed:** a portable
  software rasterizer and brand crate, single-blit presentation, glyph coverage
  lifted from the platform text engine, a run-time generated window icon, and
  per-monitor DPI awareness (Decision 0013). See `docs/RENDERER.md`.
- Implement file dialogs, external links, clipboard, notifications, and paths.
- Implement secure credential storage through the operating system.
- Add logging, crash reporting boundaries, and shutdown behavior. **Completed
  for the first host-owned in-memory diagnostic log:** a bounded closed event
  catalogue with no application input, persistence, export, or protocol
  surface (Decision 0016). Crash reporting and public/application logging
  remain separate work.
- Establish verified executable identity. **In progress:** the direct Windows
  Authenticode adapter verifies an embedded signature and returns a leaf
  certificate fingerprint (Decision 0017). It is not a package trust policy
  and does not enable product launch.

Acceptance gate: a sample application can run without Electron and exercise the
core platform services safely.

The direct Windows host creates and paints an Anodrel-owned Win32 window and
validates the core protocol shape under Decision 0006. Decision 0007 adds the
bounded framing and session engine. Decision 0008 adds the authenticated direct
Windows named-pipe adapter. Decision 0009 adds private one-time invitation
delivery. Decision 0010 adds a digest-verified, no-script application-package
text surface. A development-only Node sample now proves the full bootstrap,
authentication, and `platform.health` path over the real pipe. Remaining
acceptance work includes verified executable launch bound to an application
identity, a capability bridge, and operation-specific native tests.

## Startup Lab action tiles

The Startup Lab shows every action the platform intends to offer, each in a
declared **linked** or **planned** state (Decision 0014). This table is the
list to work through: a tile moves to linked when its documented host operation
exists and — where it is privileged — has a threat-model entry. Linking a tile
is then a data change plus its operation, not a redesign.

| Tile | State | Gate to link it |
| --- | --- | --- |
| Launch Sample | Planned | Signed package distribution and verified executable identity, bound to a validated application ID through the existing private bootstrap boundary. This is the Phase 2 acceptance item and carries the largest threat-model change; it must not be linked before that entry exists. |
| Open Logs | **Linked** | Done. Shows only the bounded typed host events defined by `docs/LOGGING.md`; it exposes no application text, persistence, export, or capability. |
| Inspect Package | **Linked** | Done. Displays facts already verified at startup; introduces no capability. |
| Runtime Diagnostics | **Linked** | Done. Displays this process's own readings; introduces no capability. |

Adding a tile beyond these four requires the same treatment: show it planned
with its gate stated, and link it only once its underlying operation is real.

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
