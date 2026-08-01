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
- Define the first direct Windows API host. **Completed:** a direct Win32
  window, JSON codec, protocol core, and lifecycle smoke test (Decision 0006).
- Define a bounded application-to-host frame and session engine. **Completed:**
  direct wire framing and host session limits (Decision 0007).
- Implement the authenticated Windows named-pipe adapter. **Completed:**
  logon-SID-restricted one-client adapter with CNG invitation (Decision 0008).
- Implement private invitation delivery. **Completed:** bounded `ANBI` record
  over a child-only inherited handle (Decision 0009).
- Define controlled application-content hosting and application identity.
  **Completed for the first no-script package surface:** strict manifest,
  canonical containment, built-in SHA-256 verification, and direct Win32 text
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
  **Completed for native Windows surfaces:** per-window immutable view
  routing, final-window shutdown, a two-window diagnostic, and the verified
  no-script text package surface (Decision 0012). A public application window
  capability remains deferred.
- Draw first-party surfaces with a software renderer. **Completed:** a portable
  software rasterizer and brand crate, single-blit presentation, glyph coverage
  lifted from the platform text engine, a run-time generated window icon, and
  per-monitor DPI awareness (Decision 0013). See `docs/RENDERER.md`.
- Define an owned native application UI foundation. **In progress:** a portable
  declarative layout tree with semantic actions establishes the first reusable
  UI contract (Decision 0025), and the Windows UI Lab renders and hit-tests a
  fixed host-owned document through that contract. Authenticated application
  input, accessibility, focus, scrolling, and an application package format
  remain separate gates.
- Establish repeatable native performance measurements. **Completed for the
  owned in-process transport and Windows named-pipe loopback paths:** a
  first-party release performance lab measures 1 KiB and 64 KiB payload latency
  with fixed warmup and documented percentile rules (Decision 0024). Startup,
  memory, rendering, and application comparisons remain separate workloads.
- Implement file dialogs, external links, clipboard, notifications, and paths.
  **Completed for the path foundation:** host-only per-application `data`,
  `cache`, and `logs` locations derived from a validated identity and the
  current user's Windows Local AppData root (Decision 0021). Filesystem
  access, directory creation, and a public storage protocol remain deferred.
- Implement secure credential storage through the operating system.
  **Completed for the credential-store foundation:** a host-only Windows
  Credential Manager adapter with per-application target isolation, bounded
  opaque secrets, and current-user local persistence (Decision 0022). Public
  credential capabilities, session binding, and consent remain deferred.
- Add logging, crash reporting boundaries, and shutdown behavior. **Completed
  for the first in-memory diagnostic log:** a bounded closed event
  catalogue with no application input, persistence, export, or protocol
  surface (Decision 0016). Crash reporting and public/application logging
  remain separate work.
- Establish verified executable identity. **In progress:** the direct Windows
  Authenticode adapter verifies an embedded signature and returns a leaf
  certificate fingerprint (Decision 0017). The installed application-record
  foundation now binds the expected executable digest and publisher fingerprint
  to a validated package identity outside the package directory (Decision
  0018). The direct Windows policy adapter now reads that record only from the
  machine-wide 64-bit registry (Decision 0019). Record provisioning and
  Startup Lab integration remain required. The direct launch service now locks,
  revalidates, verifies, and tracks a policy-approved executable before
  delivering bootstrap material (Decision 0020); it has no installed sample to
  launch yet.

Acceptance gate: a sample application can run without Electron and exercise the
core platform services safely.

The direct Windows host creates and paints an Anodrel Win32 window and
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

- A full native UI toolkit beyond the constrained foundation in Decision 0025.
- Custom browser engine.
- Custom operating system.
- Full Anodex rewrite before the platform contracts are proven.
