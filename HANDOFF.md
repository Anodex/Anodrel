# Anodrel Project Handoff

**Date:** 2026-07-31  
**Project path:** C:\Users\Owner\Desktop\Platform X  
**Status:** Foundation repository created; implementation has not started.

## What this project is

Anodrel is a standalone, reusable native application platform intended to
replace Electron for applications that need desktop windows, operating-system
services, permissions, secure storage, process management, and a controlled
application/runtime boundary.

Anodex is the first planned application that may use Anodrel. Anodex is not
part of this repository and has not been modified.

The final product name is intentionally undecided. “Anodrel” is only a
working name.

## What has been created

This repository has its own Git history root and currently contains:

- README.md — project purpose, goals, boundaries, and repository map.
- AGENTS.md — rules for future coding agents.
- ROADMAP.md — staged implementation plan and acceptance gates.
- docs/ARCHITECTURE.md — layer model, responsibilities, protocol, security,
  migration, and testing strategy.
- docs/DEVELOPMENT.md — development and documentation workflow.
- docs/decisions/0001-standalone-repository.md — decision to keep Anodrel
  separate from Anodex.
- docs/decisions/README.md — decision-record format.
- apps/README.md — application ownership.
- packages/README.md — reusable package ownership.
- native/README.md — native host ownership.
- scripts/README.md — script requirements.
- tests/README.md — cross-component test ownership.
- .gitignore — exclusions for source control, build output, secrets, logs, and
  local runtime data.

## Current architectural direction

The current direction is a layered platform:

~~~text
Application
    │
    │ versioned SDK/protocol
    ▼
Platform Core
    │
    │ platform service interfaces
    ▼
Native Host
    │
    ├── Windows first
    ├── macOS later
    └── Linux later
~~~

The expected implementation direction is:

- Rust for the native host and platform-sensitive code.
- TypeScript packages for reusable protocol types and SDKs where practical.
- Existing React applications can remain UI clients initially.
- A versioned protocol separates applications from the native host.
- Electron may remain an Anodex adapter temporarily during migration.

These are design directions, not all final implementation decisions. Open
decisions are listed in docs/ARCHITECTURE.md.

## Non-negotiable boundaries

- Keep Anodrel in its own repository.
- Do not copy Anodex source into Anodrel.
- Do not add Anodrel files to the Anodex repository.
- Keep Anodex-specific concepts out of the platform core.
- Do not begin a large Anodex migration before platform contracts are stable.
- Do not expose arbitrary native access to application content.
- Document substantial architecture, protocol, and security changes.

## Immediate next milestone

Complete Phase 0 from ROADMAP.md:

1. Choose Windows as the first supported platform.
2. Decide the first native host language/framework.
3. Decide whether the first UI uses a system webview or a native UI.
4. Decide the first local transport.
5. Write the threat model before exposing filesystem, process, or credential
   capabilities.
6. Record those choices as numbered decision records.

The first implementation should be a small sample application and mock host,
not an Anodex migration.

## Recommended first implementation sequence

1. Define protocol envelopes, request IDs, typed errors, events, and
   cancellation.
2. Define platform capability interfaces.
3. Build a mock host for contract tests.
4. Build a minimal Windows native host.
5. Exercise lifecycle, window creation, logging, and shutdown.
6. Add one capability at a time: paths, dialogs, external links, clipboard,
   secure storage, notifications, and child processes.
7. Build a small sample application against the public interface.
8. Only then design the Anodex adapter.

## How to resume this project

Open this folder as its own workspace:

~~~text
C:\Users\Owner\Desktop\Platform X
~~~

Read these files in order:

1. HANDOFF.md
2. README.md
3. ROADMAP.md
4. docs/ARCHITECTURE.md
5. docs/DEVELOPMENT.md
6. docs/decisions/

Before implementation begins, review the open decisions and update the roadmap
with the selected first milestone. The repository currently has uncommitted
foundation files and has not been pushed or published.

## Relationship to Anodex

Anodex remains at:

~~~text
C:\Users\Owner\Desktop\Anodex4
~~~

It should continue to build and operate independently. The future integration
should use a documented Anodrel adapter and should be introduced only after
the platform has a working host, contract tests, recovery behavior, and a
rollback plan.
