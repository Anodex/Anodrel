# Anodrel roadmap

Anodrel is intentionally staged. The platform must be useful, secure, and
stable before Anodex or another application depends on it.

## Delivery order

Windows is the reference platform. It must meet its
[release gates](docs/WINDOWS_RELEASE.md) before new Linux-specific or
macOS-specific feature work begins. Portable work may continue only when it
directly closes a Windows release gate.

This sequence prevents three partial desktop hosts from being mistaken for a
finished platform:

1. Finish and ship a credible Windows reference release.
2. Port its proven contracts to Linux.
3. Port those contracts to macOS.

## Phase 0 — Foundation

**Status:** In progress; core architecture is established.

Completed foundation work:

- Anodrel is a standalone repository with clear folders, documentation rules,
  and no copied Anodex source.
- The core layer model, security boundaries, and first Windows implementation
  are documented.
- The first direct Win32 host, bounded frame/session engine, authenticated
  Windows named pipe, and child-only invitation delivery are implemented.
- The first strict no-script application package has canonical containment,
  content digest verification, and native rendering.

The phase acceptance standard is met for the initial architecture: the platform
has a documented first milestone and no dependency on Anodex source.

## Phase 1 — Contracts and protocol

**Status:** Foundation slice in progress.

Implemented foundation:

- versioned envelopes, request identifiers, errors, cancellation, and
  capability grants;
- typed SDKs, mock host, sample application, compatibility tests, and bounded
  native-session engine; and
- documented compatibility and security rules for public protocol changes.

The full breadth of future desktop services will be added only with explicit
authority, threat model, contract, and verification. The established protocol
and development samples already meet the initial acceptance standard: a small
sample can use documented host interfaces without native internals.

## Phase 2 — Native host

**Status:** Direct Windows host in progress.

The Windows foundation contains direct native lifecycle, presentation,
authenticated transport, semantic UI, accessibility, platform services,
performance evidence, verified executable launch, and owned installer/update
foundations. Its release work remains in progress, rather than being declared
complete based on development diagnostics alone.

See [Phase 2: Native host](docs/roadmap/NATIVE_HOST.md) for the detailed
implementation map. See [Windows release readiness](docs/WINDOWS_RELEASE.md)
for the exact release gates, current evidence, manual desktop proof, and
production decisions.

### Current Windows release focus

- Complete visible desktop acceptance for the documented feature set.
- Run and record the signed development fixture install, launcher, update,
  recovery, and cleanup paths when an operator explicitly authorizes the
  temporary machine-trust change.
- Select production certificate custody, renewal, timestamp, and release
  operation; then prove the production distribution path.
- Add reusable UI or service capability only when the first real application
  establishes the need and its contract can remain narrow.

### Linux and macOS

Linux has limited direct development foundations—private transport,
invited-child delivery, bounded state and crash stores, and a fixed Wayland
diagnostic—but no application host, product identity, installation, updates, or
accessibility provider. macOS implementation has not begun. Both wait for the
Windows reference release.

## Phase 3 — Reusable SDK and tooling

**Status:** First starter-package slice in progress.

Implemented:

- a transport-neutral TypeScript SDK and in-repository native Windows UI SDK;
- constrained first-party native templates for the core UI, menus, context
  menus, multiple windows, forms, live status, scrolling, notifications, file
  output, and window controls;
- fixed-grant development-host routes, isolated release builds, and real
  invited-child session checks for those templates; and
- a native text-package tool, static desktop example, and command-line example.

The Windows development-template acceptance gate is met: a project can be
created from a documented template and run without native-host internals.
Manual desktop checks, production executable identity and packaging, publication,
and non-Windows hosts remain distinct release work.

## Future work

Phase 4, Phase 5, and deliberately deferred product work are maintained in
[Future roadmap](docs/roadmap/FUTURE.md). They are not current delivery work
until the Windows release gates above close.

## How work enters the roadmap

A proposed capability must answer four questions before implementation:

1. Which application need makes it necessary?
2. Which layer owns it and what explicit authority crosses the boundary?
3. What safety, lifecycle, performance, and compatibility limits apply?
4. Which focused automated and visible manual checks prove it?

Important answers become decision records under `docs/decisions/`. This keeps
Anodrel modular and avoids growing a browser-runtime-shaped surface by accident.
