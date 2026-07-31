# Anodrel Project Handoff

**Date:** 2026-07-31  
**Project path:** C:\Users\Owner\Desktop\Platform X  
**Status:** Protocol-first foundation implementation underway; owned wire,
authenticated named-pipe, private child bootstrap, direct Windows host, and
strict application package modules prove bounded protocol handling, local
session protection, private credential delivery, window lifecycle, a
development-only end-to-end health probe, a digest-verified no-script text
surface, and a branded native Startup Lab visual smoke test.

## What this project is

Anodrel is a standalone, reusable native application platform intended to
replace Electron for applications that need desktop windows, operating-system
services, permissions, secure storage, process management, and a controlled
application/runtime boundary.

Anodex is the first planned application that may use Anodrel. Anodex is not
part of this repository and has not been modified.

The product name is Anodrel. The local workspace still uses its original folder
name while this task is open.

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
- package.json and TypeScript project configuration — workspace build and
  verification commands.
- packages/protocol — versioned JSON-compatible protocol types and validation.
- packages/sdk — application-facing client over an abstract transport.
- packages/mock-host — policy-driven in-memory host for contract tests.
- apps/sample — a small application using only the public SDK.
- tests/contract — protocol compatibility checks shared with future hosts.
- docs/PROTOCOL.md, docs/TRANSPORT.md, docs/THREAT_MODEL.md, and
  docs/PERFORMANCE.md — public contracts, security baseline, and performance
  measurement rules for future native work.
- native/ — owned Rust JSON, protocol, core, wire, transport, authenticated
  Windows named-pipe, and direct Win32 window modules;
  the deployed dependency graph contains no third-party runtime library.

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

Continue from Decision 0010 with the next native-host security boundary:

1. Define signed package distribution and verified executable identity.
2. Bind a verified executable session to its validated application ID without
   exposing bootstrap material.
3. Define a narrow capability bridge and extend the threat model before any
   filesystem, process, or credential capability.

The initial implementation is a small sample application and mock host, not an
Anodex migration. Its v1 protocol is documented in docs/PROTOCOL.md.

## Recommended first implementation sequence

1. Define package signing and bind a verified executable identity to the
   authenticated direct Windows named-pipe adapter through the existing private
   bootstrap boundary.
2. Exercise lifecycle, window creation, logging, and shutdown through that
   bound transport.
4. Add one capability at a time: paths, dialogs, external links, clipboard,
   secure storage, notifications, and child processes.
5. Run the shared contract suite against the native host and add native
   integration and manual tests.
6. Only then design the Anodex adapter.

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
6. docs/PROTOCOL.md
7. docs/TRANSPORT.md
8. docs/PERFORMANCE.md
9. docs/THREAT_MODEL.md
10. docs/decisions/

Before adding application content or privileged native behavior, review the
open decisions and extend the threat model. The foundation is published to the
`Anodex/Anodrel` repository; check Git status before resuming work.

## Relationship to Anodex

Anodex remains at:

~~~text
C:\Users\Owner\Desktop\Anodex4
~~~

It should continue to build and operate independently. The future integration
should use a documented Anodrel adapter and should be introduced only after
the platform has a working host, contract tests, recovery behavior, and a
rollback plan.
